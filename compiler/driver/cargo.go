package driver

// Cargo-driven builds: the crate graph comes from `cargo metadata`; every
// crate is compiled by rustgo itself (build scripts and proc-macro servers
// too), and the whole program is emitted as one C program (compiled as one
// or, when large, several translation units: see compileC).

import (
	"encoding/json"
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"sort"
	"strings"

	"rustgo/compiler/ast"
	"rustgo/compiler/codegen"
	"rustgo/compiler/expand"
	. "rustgo/compiler/middle"
	"rustgo/compiler/resolve"
	"rustgo/compiler/typeck"
)

// cargo metadata JSON (subset)
type cargoPkg struct {
	ID          string   `json:"id"`
	Name        string   `json:"name"`
	Version     string   `json:"version"`
	Edition     string   `json:"edition"`
	Manifest    string   `json:"manifest_path"`
	Authors     []string `json:"authors"`
	Description *string  `json:"description"`
	Homepage    *string  `json:"homepage"`
	Repository  *string  `json:"repository"`
	License     *string  `json:"license"`
	Targets     []struct {
		Name    string   `json:"name"`
		Kind    []string `json:"kind"`
		SrcPath string   `json:"src_path"`
		Edition string   `json:"edition"`
	} `json:"targets"`
}

type cargoDep struct {
	Name     string `json:"name"`
	Pkg      string `json:"pkg"`
	DepKinds []struct {
		Kind *string `json:"kind"`
	} `json:"dep_kinds"`
}

type cargoMeta struct {
	Packages []*cargoPkg `json:"packages"`
	Resolve  struct {
		Root  string `json:"root"`
		Nodes []struct {
			ID       string     `json:"id"`
			Deps     []cargoDep `json:"deps"`
			Features []string   `json:"features"`
		} `json:"nodes"`
	} `json:"resolve"`
	WorkspaceMembers []string `json:"workspace_members"`
}

// crateUnit is one crate to compile.
type crateUnit struct {
	key      string // unique internal crate name
	name     string // crate name (underscored)
	src      string
	edition  string
	features []string
	deps     map[string]string // extern name -> unit key
	procMac  bool
	pkg      *cargoPkg
	// build script of this package (nil if none)
	buildScript *crateUnit
	// results of running the build script
	cfgs []string
	env  map[string]string
	link []string
}

// CargoOptions configure a Cargo-driven build.
type CargoOptions struct {
	Manifest string
	Bin      string
}

func hasKind(kinds []string, k string) bool {
	for _, x := range kinds {
		if x == k {
			return true
		}
	}
	return false
}

func depKind(d cargoDep, kind string) bool {
	for _, k := range d.DepKinds {
		if (k.Kind == nil && kind == "") || (k.Kind != nil && *k.Kind == kind) {
			return true
		}
	}
	return false
}

// planCargo runs `cargo metadata` and computes the crate graph in
// dependency order (dependencies first; the binary last).
func planCargo(co *CargoOptions) ([]*crateUnit, error) {
	args := []string{"metadata", "--offline", "--format-version", "1", "--filter-platform", "x86_64-unknown-linux-gnu"}
	if co.Manifest != "" {
		args = append(args, "--manifest-path", co.Manifest)
	}
	cmd := exec.Command("cargo", args...)
	cmd.Stderr = os.Stderr
	out, err := cmd.Output()
	if err != nil {
		return nil, fmt.Errorf("cargo metadata failed: %v", err)
	}
	var meta cargoMeta
	if err := json.Unmarshal(out, &meta); err != nil {
		return nil, err
	}
	pkgs := map[string]*cargoPkg{}
	for _, p := range meta.Packages {
		pkgs[p.ID] = p
	}
	nodes := map[string]int{}
	for i, n := range meta.Resolve.Nodes {
		nodes[n.ID] = i
	}
	root := meta.Resolve.Root
	if root == "" && len(meta.WorkspaceMembers) > 0 {
		root = meta.WorkspaceMembers[0]
	}
	nameCount := map[string]int{}
	for _, n := range meta.Resolve.Nodes {
		nameCount[pkgs[n.ID].Name]++
	}
	unitOf := map[string]*crateUnit{}
	var order []*crateUnit
	var visit func(id string) (*crateUnit, error)
	visit = func(id string) (*crateUnit, error) {
		if u, ok := unitOf[id]; ok {
			return u, nil
		}
		p := pkgs[id]
		n := meta.Resolve.Nodes[nodes[id]]
		u := &crateUnit{pkg: p, edition: p.Edition, features: n.Features, deps: map[string]string{}}
		found := false
		var buildSrc, buildEd string
		for _, t := range p.Targets {
			if !found && (hasKind(t.Kind, "lib") || hasKind(t.Kind, "rlib") || hasKind(t.Kind, "proc-macro")) {
				u.name = strings.ReplaceAll(t.Name, "-", "_")
				u.src = t.SrcPath
				if t.Edition != "" {
					u.edition = t.Edition
				}
				u.procMac = hasKind(t.Kind, "proc-macro")
				found = true
			}
			if hasKind(t.Kind, "custom-build") {
				buildSrc, buildEd = t.SrcPath, t.Edition
			}
		}
		if !found {
			return nil, nil
		}
		u.key = u.name
		if nameCount[p.Name] > 1 {
			u.key = u.name + "__v" + strings.NewReplacer(".", "_", "-", "_", "+", "_").Replace(p.Version)
		}
		unitOf[id] = u
		if buildSrc != "" {
			bs := &crateUnit{key: u.key + "__build_script", name: "build_script_build", src: buildSrc, edition: buildEd, features: n.Features, deps: map[string]string{}, pkg: p}
			if bs.edition == "" {
				bs.edition = p.Edition
			}
			for _, d := range n.Deps {
				if depKind(d, "build") {
					du, err := visit(d.Pkg)
					if err != nil {
						return nil, err
					}
					if du != nil {
						bs.deps[d.Name] = du.key
					}
				}
			}
			u.buildScript = bs
		}
		for _, d := range n.Deps {
			if !depKind(d, "") {
				continue
			}
			du, err := visit(d.Pkg)
			if err != nil {
				return nil, err
			}
			if du != nil {
				u.deps[d.Name] = du.key
			}
		}
		order = append(order, u)
		return u, nil
	}
	rp := pkgs[root]
	rn := meta.Resolve.Nodes[nodes[root]]
	lib, err := visit(root)
	if err != nil {
		return nil, err
	}
	var bin *crateUnit
	for _, t := range rp.Targets {
		if hasKind(t.Kind, "bin") && (co.Bin == "" || t.Name == co.Bin) {
			bin = &crateUnit{key: strings.ReplaceAll(t.Name, "-", "_"), name: strings.ReplaceAll(t.Name, "-", "_"), src: t.SrcPath, edition: rp.Edition, features: rn.Features, deps: map[string]string{}, pkg: rp}
			if t.Edition != "" {
				bin.edition = t.Edition
			}
			break
		}
	}
	if bin == nil {
		return nil, fmt.Errorf("no binary target found in %s", rp.Manifest)
	}
	for _, d := range rn.Deps {
		if depKind(d, "") {
			if du, _ := visit(d.Pkg); du != nil {
				bin.deps[d.Name] = du.key
			}
		}
	}
	if lib != nil {
		bin.deps[lib.name] = lib.key
		if lib.key == bin.key {
			bin.key = bin.key + "__bin"
		}
		// the package's build script outputs apply to the binary too
		bin.buildScript = nil
	} else {
		for _, t := range rp.Targets {
			if hasKind(t.Kind, "custom-build") {
				bsu := &crateUnit{key: bin.key + "__build_script", name: "build_script_build", src: t.SrcPath, edition: rp.Edition, deps: map[string]string{}, pkg: rp, features: rn.Features}
				for _, d := range rn.Deps {
					if depKind(d, "build") {
						if du, _ := visit(d.Pkg); du != nil {
							bsu.deps[d.Name] = du.key
						}
					}
				}
				bin.buildScript = bsu
			}
		}
	}
	order = append(order, bin)
	return order, nil
}

// cargoEnv returns the compile-time environment Cargo provides to a crate.
func cargoEnv(u *crateUnit) map[string]string {
	env := map[string]string{}
	p := u.pkg
	if p == nil {
		return env
	}
	env["CARGO_PKG_NAME"] = p.Name
	env["CARGO_PKG_VERSION"] = p.Version
	ver := p.Version
	pre := ""
	if i := strings.IndexByte(ver, '-'); i >= 0 {
		ver, pre = ver[:i], ver[i+1:]
	}
	parts := strings.SplitN(ver, ".", 3)
	for len(parts) < 3 {
		parts = append(parts, "0")
	}
	env["CARGO_PKG_VERSION_MAJOR"], env["CARGO_PKG_VERSION_MINOR"], env["CARGO_PKG_VERSION_PATCH"] = parts[0], parts[1], parts[2]
	env["CARGO_PKG_VERSION_PRE"] = pre
	env["CARGO_PKG_AUTHORS"] = strings.Join(p.Authors, ":")
	str := func(s *string) string {
		if s == nil {
			return ""
		}
		return *s
	}
	env["CARGO_PKG_DESCRIPTION"] = str(p.Description)
	env["CARGO_PKG_HOMEPAGE"] = str(p.Homepage)
	env["CARGO_PKG_REPOSITORY"] = str(p.Repository)
	env["CARGO_PKG_LICENSE"] = str(p.License)
	env["CARGO_MANIFEST_DIR"] = filepath.Dir(p.Manifest)
	env["CARGO_CRATE_NAME"] = u.name
	for k, v := range u.env {
		env[k] = v
	}
	return env
}

// buildState carries shared state of a Cargo build.
type buildState struct {
	opts     *Options
	byKey    map[string]*crateUnit
	pmMacros map[string]map[string]*expand.ProcMacro
	outDirs  string
}

// BuildCargo compiles the binary of a Cargo package with all its
// dependencies.
func BuildCargo(opts *Options, co *CargoOptions) error {
	units, err := planCargo(co)
	if err != nil {
		return err
	}
	bs := &buildState{opts: opts, byKey: map[string]*crateUnit{}, pmMacros: map[string]map[string]*expand.ProcMacro{}}
	for _, u := range units {
		bs.byKey[u.key] = u
	}
	bs.outDirs, _ = os.MkdirTemp("", "rustgo-out-")
	defer os.RemoveAll(bs.outDirs)
	defer func() {
		for _, ms := range bs.pmMacros {
			for _, m := range ms {
				m.Server.Close()
			}
		}
	}()
	// Build scripts and proc-macro servers first, in dependency order.
	for _, u := range units {
		if u.buildScript != nil {
			if err := bs.runBuildScript(u); err != nil {
				return fmt.Errorf("build script of %s: %v", u.pkg.Name, err)
			}
		}
		if u.procMac {
			ms, err := buildProcMacroServer(opts, u, bs.byKey, units, bs.pmMacros)
			if err != nil {
				return fmt.Errorf("proc-macro crate %s: %v", u.name, err)
			}
			bs.pmMacros[u.key] = ms
		}
	}
	// the binary shares its package's build-script results
	bin := units[len(units)-1]
	if lib := bs.byKey[bin.deps[strings.ReplaceAll(bin.pkg.Name, "-", "_")]]; lib != nil && lib.pkg == bin.pkg {
		bin.cfgs, bin.env = lib.cfgs, lib.env
	}
	src, err := bs.compileProgram(units, len(units)-1)
	if err != nil {
		return err
	}
	for _, u := range units {
		opts.LinkArgs = append(opts.LinkArgs, u.link...)
	}
	return compileC(opts, src)
}

// compileProgram compiles units[binIdx] with its dependencies (taken from
// units, which are in dependency order) and returns C source.
func (bs *buildState) compileProgram(units []*crateUnit, binIdx int) (*codegen.Program, error) {
	need := map[string]bool{}
	var mark func(u *crateUnit)
	mark = func(u *crateUnit) {
		if need[u.key] {
			return
		}
		need[u.key] = true
		for _, k := range u.deps {
			if du := bs.byKey[k]; du != nil {
				mark(du)
			}
		}
	}
	mark(units[binIdx])
	exp := expand.New()
	stdCrate, err := loadStd(bs.opts)
	if err != nil {
		return nil, err
	}
	if err := exp.ExpandCrate(stdCrate, "std"); err != nil {
		return nil, fmt.Errorf("while expanding std: %v", err)
	}
	r := resolve.New()
	r.IsBuiltinMacro = expand.IsBuiltinMacro
	r.IsCrateMacro = exp.IsCrateMacro
	r.AddCrate("std", stdCrate)
	crates := map[string]*Crate{}
	var binCrate *Crate
	for i, u := range units {
		if !need[u.key] {
			continue
		}
		if u.procMac {
			crates[u.key] = procMacroStub(r, u, bs.pmMacros[u.key])
			continue
		}
		c, err := loadUnitAST(u)
		if err != nil {
			return nil, err
		}
		if i == binIdx {
			addMainWrapper(c)
		}
		exp.Config = &expand.CrateConfig{Features: u.features, Cfgs: append(append([]string{}, u.cfgs...), globalCfgs...), Env: cargoEnv(u)}
		exp.Externs = u.deps
		exp.ProcMacros = map[string]*expand.ProcMacro{}
		// proc macros of direct deps, plus those a dependency re-exports
		// from its own deps (`pub use ctor_proc_macro::ctor;`); direct
		// ones win on name clashes
		for _, k := range sortedKeys(u.deps) {
			if du := bs.byKey[u.deps[k]]; du != nil {
				for _, kk := range sortedKeys(du.deps) {
					for n, m := range bs.pmMacros[du.deps[kk]] {
						exp.ProcMacros[n] = m
					}
				}
			}
		}
		for _, k := range u.deps {
			for n, m := range bs.pmMacros[k] {
				exp.ProcMacros[n] = m
			}
		}
		if err := exp.ExpandCrate(c, u.key); err != nil {
			return nil, fmt.Errorf("crate %s: %v", u.name, err)
		}
		exp.Config = nil
		exp.Externs = nil
		exp.ProcMacros = nil
		cr := r.AddCrate(u.key, c)
		cr.Edition = u.edition
		if cr.Externs == nil {
			// (may already hold `extern crate x as y;` aliases)
			cr.Externs = map[string]*Crate{}
		}
		for _, n := range sortedKeys(u.deps) {
			if dc := crates[u.deps[n]]; dc != nil {
				cr.Externs[n] = dc
			}
		}
		crates[u.key] = cr
		if i == binIdx {
			binCrate = cr
		}
	}
	r.ResolveImports()
	if err := r.Err(); err != nil {
		return nil, err
	}
	tcx := typeck.NewTcx(r)
	entry := "main"
	if b := binCrate.Root.Lookup("__rustgo_main", ValueNS); b != nil {
		entry = "__rustgo_main"
	}
	b := binCrate.Root.Lookup(entry, ValueNS)
	if b == nil || b.Res.Def == nil || b.Res.Def.Kind != DefFn {
		return nil, fmt.Errorf("error: `main` function not found in crate %s", binCrate.Name)
	}
	if aborted := tcx.Guard(func() { tcx.BuildImpls() }); aborted || tcx.Err() != nil {
		return nil, tcx.Err()
	}
	g := codegen.New(tcx)
	var prog *codegen.Program
	var genErr error
	if aborted := tcx.Guard(func() { prog, genErr = g.GenerateProgram(b.Res.Def) }); aborted {
		return nil, tcx.Err()
	}
	if genErr != nil {
		return nil, genErr
	}
	if err := tcx.Err(); err != nil {
		return nil, err
	}
	return prog, nil
}

// procMacroStub registers a proc-macro crate as an empty crate whose root
// binds its macro names (so `use pm::Derive;` resolves).
func procMacroStub(r *resolve.Resolver, u *crateUnit, ms map[string]*expand.ProcMacro) *Crate {
	stub := r.AddCrate(u.key, &ast.Crate{})
	stub.Edition = u.edition
	names := make([]string, 0, len(ms))
	for n := range ms {
		names = append(names, n)
	}
	sort.Strings(names)
	for _, n := range names {
		md := NewDef(DefMod, n, stub.Def)
		md.Mod = NewModule(md, stub.Root, stub, false)
		stub.Root.Define(n, TypeNS, &NameBinding{Res: Res{Kind: ResDef, Def: md}})
	}
	return stub
}

// runBuildScript compiles and runs a package's build script, recording the
// cfgs, environment and link flags it emits on the package's unit.
func (bs *buildState) runBuildScript(u *crateUnit) error {
	script := u.buildScript
	outDir := filepath.Join(bs.outDirs, u.key)
	os.MkdirAll(outDir, 0o755)
	units := []*crateUnit{}
	seen := map[string]bool{}
	var add func(k string)
	add = func(k string) {
		if seen[k] {
			return
		}
		seen[k] = true
		du := bs.byKey[k]
		if du == nil {
			return
		}
		for _, dk := range sortedKeys(du.deps) {
			add(du.deps[dk])
		}
		units = append(units, du)
	}
	for _, n := range sortedKeys(script.deps) {
		add(script.deps[n])
	}
	units = append(units, script)
	bs.byKey[script.key] = script
	src, err := bs.compileProgram(units, len(units)-1)
	delete(bs.byKey, script.key)
	if err != nil {
		return err
	}
	exe := filepath.Join(outDir, "build-script")
	popts := *bs.opts
	popts.Output = exe
	popts.EmitC = ""
	popts.LinkArgs = nil
	if err := compileC(&popts, src); err != nil {
		return err
	}
	cmd := exec.Command(exe)
	cmd.Dir = filepath.Dir(u.pkg.Manifest)
	env := os.Environ()
	for k, v := range cargoEnv(u) {
		env = append(env, k+"="+v)
	}
	rustc, _ := exec.LookPath("rustc")
	if rustc == "" {
		rustc = "rustc"
	}
	env = append(env,
		"OUT_DIR="+outDir, "TARGET=x86_64-unknown-linux-gnu", "HOST=x86_64-unknown-linux-gnu",
		"PROFILE=debug", "OPT_LEVEL=0", "DEBUG=true", "NUM_JOBS=4", "RUSTC="+rustc,
		"CARGO_CFG_TARGET_OS=linux", "CARGO_CFG_TARGET_ARCH=x86_64", "CARGO_CFG_TARGET_FAMILY=unix",
		"CARGO_CFG_UNIX=", "CARGO_CFG_TARGET_POINTER_WIDTH=64", "CARGO_CFG_TARGET_ENDIAN=little",
		"CARGO_CFG_TARGET_ENV=gnu", "CARGO_CFG_TARGET_VENDOR=unknown", "CARGO_CFG_TARGET_HAS_ATOMIC=8,16,32,64,ptr",
		"CARGO_CFG_PANIC=unwind", "CARGO_CFG_DEBUG_ASSERTIONS=", "CARGO_ENCODED_RUSTFLAGS=--cfg\x1frustix_use_libc", "CARGO=cargo",
		"CARGO_CFG_RUSTIX_USE_LIBC=",
	)
	for _, f := range u.features {
		env = append(env, "CARGO_FEATURE_"+strings.ToUpper(strings.ReplaceAll(f, "-", "_"))+"=1")
	}
	cmd.Env = env
	cmd.Stderr = os.Stderr
	out, err := cmd.Output()
	if err != nil {
		return fmt.Errorf("build script failed: %v\n%s", err, truncate(string(out), 2000))
	}
	u.env = map[string]string{"OUT_DIR": outDir}
	for _, line := range strings.Split(string(out), "\n") {
		line = strings.TrimSpace(line)
		var rest string
		switch {
		case strings.HasPrefix(line, "cargo::"):
			rest = line[len("cargo::"):]
		case strings.HasPrefix(line, "cargo:"):
			rest = line[len("cargo:"):]
		default:
			continue
		}
		key, val, _ := strings.Cut(rest, "=")
		switch key {
		case "rustc-cfg":
			u.cfgs = append(u.cfgs, val)
		case "rustc-env":
			if k, v, ok := strings.Cut(val, "="); ok {
				u.env[k] = v
			}
		case "rustc-link-lib":
			if _, lib, ok := strings.Cut(val, "="); ok {
				val = lib
			}
			u.link = append(u.link, "-l"+val)
		case "rustc-link-search":
			if _, p, ok := strings.Cut(val, "="); ok {
				val = p
			}
			u.link = append(u.link, "-L"+val)
		case "rustc-link-arg":
			u.link = append(u.link, val)
		}
	}
	return nil
}

// globalCfgs are passed to every crate (like RUSTFLAGS="--cfg ..."):
// rustix's default linux_raw backend needs inline assembly, which we don't
// support, so select its libc backend.
var globalCfgs = []string{"rustix_use_libc"}
