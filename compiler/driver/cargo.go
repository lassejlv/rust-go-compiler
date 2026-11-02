package driver

import (
	"sort"
	"encoding/json"
	"fmt"
	"os"
	"os/exec"
	"strings"

	"rustgo/compiler/ast"
	"rustgo/compiler/codegen"
	"rustgo/compiler/expand"
	. "rustgo/compiler/middle"
	"rustgo/compiler/resolve"
	"rustgo/compiler/typeck"
)

// cargo metadata JSON (subset)
type cargoMeta struct {
	Packages []struct {
		ID       string `json:"id"`
		Name     string `json:"name"`
		Version  string `json:"version"`
		Edition  string `json:"edition"`
		Manifest string `json:"manifest_path"`
		Targets  []struct {
			Name    string   `json:"name"`
			Kind    []string `json:"kind"`
			SrcPath string   `json:"src_path"`
			Edition string   `json:"edition"`
		} `json:"targets"`
	} `json:"packages"`
	Resolve struct {
		Root  string `json:"root"`
		Nodes []struct {
			ID   string `json:"id"`
			Deps []struct {
				Name     string `json:"name"`
				Pkg      string `json:"pkg"`
				DepKinds []struct {
					Kind *string `json:"kind"`
				} `json:"dep_kinds"`
			} `json:"deps"`
			Features []string `json:"features"`
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
	pkgID    string
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
	pkgs := map[string]int{}
	for i, p := range meta.Packages {
		pkgs[p.ID] = i
	}
	nodes := map[string]int{}
	for i, n := range meta.Resolve.Nodes {
		nodes[n.ID] = i
	}
	root := meta.Resolve.Root
	if root == "" && len(meta.WorkspaceMembers) > 0 {
		root = meta.WorkspaceMembers[0]
	}
	// count names to detect duplicates (multiple versions)
	nameCount := map[string]int{}
	for _, n := range meta.Resolve.Nodes {
		nameCount[meta.Packages[pkgs[n.ID]].Name]++
	}
	unitOf := map[string]*crateUnit{} // pkg id -> lib unit
	var order []*crateUnit
	var visit func(id string) (*crateUnit, error)
	visit = func(id string) (*crateUnit, error) {
		if u, ok := unitOf[id]; ok {
			return u, nil
		}
		p := meta.Packages[pkgs[id]]
		n := meta.Resolve.Nodes[nodes[id]]
		u := &crateUnit{pkgID: id, edition: p.Edition, features: n.Features, deps: map[string]string{}}
		found := false
		for _, t := range p.Targets {
			if hasKind(t.Kind, "lib") || hasKind(t.Kind, "rlib") || hasKind(t.Kind, "proc-macro") {
				u.name = strings.ReplaceAll(t.Name, "-", "_")
				u.src = t.SrcPath
				if t.Edition != "" {
					u.edition = t.Edition
				}
				u.procMac = hasKind(t.Kind, "proc-macro")
				found = true
				break
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
		for _, d := range n.Deps {
			normal := false
			for _, k := range d.DepKinds {
				if k.Kind == nil {
					normal = true
				}
			}
			if !normal {
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
	rp := meta.Packages[pkgs[root]]
	rn := meta.Resolve.Nodes[nodes[root]]
	// dependencies of the root package
	lib, err := visit(root)
	if err != nil {
		return nil, err
	}
	// the binary target
	var bin *crateUnit
	for _, t := range rp.Targets {
		if hasKind(t.Kind, "bin") && (co.Bin == "" || t.Name == co.Bin) {
			bin = &crateUnit{key: strings.ReplaceAll(t.Name, "-", "_"), name: strings.ReplaceAll(t.Name, "-", "_"), src: t.SrcPath, edition: rp.Edition, features: rn.Features, deps: map[string]string{}}
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
		for _, k := range d.DepKinds {
			if k.Kind == nil {
				if du, _ := visit(d.Pkg); du != nil {
					bin.deps[d.Name] = du.key
				}
			}
		}
	}
	if lib != nil {
		bin.deps[lib.name] = lib.key
		if lib.key == bin.key {
			bin.key = bin.key + "__bin"
		}
	}
	order = append(order, bin)
	return order, nil
}

// BuildCargo compiles the binary of a Cargo package with all its
// dependencies.
func BuildCargo(opts *Options, co *CargoOptions) error {
	units, err := planCargo(co)
	if err != nil {
		return err
	}
	exp := expand.New()
	stdCrate, err := loadStd(opts)
	if err != nil {
		return err
	}
	if err := exp.ExpandCrate(stdCrate, "std"); err != nil {
		return fmt.Errorf("while expanding std: %v", err)
	}
	r := resolve.New()
	r.IsBuiltinMacro = expand.IsBuiltinMacro
	r.AddCrate("std", stdCrate)
	crates := map[string]*Crate{}
	byKey := map[string]*crateUnit{}
	for _, u := range units {
		byKey[u.key] = u
	}
	pmMacros := map[string]map[string]*expand.ProcMacro{}
	var binCrate *Crate
	for i, u := range units {
		if u.procMac {
			ms, err := buildProcMacroServer(opts, u, byKey, units)
			if err != nil {
				return fmt.Errorf("proc-macro crate %s: %v", u.name, err)
			}
			pmMacros[u.key] = ms
			// a stub crate so that `use pm_crate::macro_name;` resolves
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
			crates[u.key] = stub
			continue
		}
		c, err := loadUnitAST(u)
		if err != nil {
			return err
		}
		if i == len(units)-1 {
			addMainWrapper(c)
		}
		exp.Config = &expand.CrateConfig{Features: u.features}
		exp.ProcMacros = map[string]*expand.ProcMacro{}
		for _, k := range u.deps {
			for n, m := range pmMacros[k] {
				exp.ProcMacros[n] = m
			}
		}
		if err := exp.ExpandCrate(c, u.key); err != nil {
			return fmt.Errorf("crate %s: %v", u.name, err)
		}
		exp.Config = nil
		exp.ProcMacros = nil
		cr := r.AddCrate(u.key, c)
		cr.Edition = u.edition
		cr.Externs = map[string]*Crate{}
		for _, n := range sortedKeys(u.deps) {
			if dc := crates[u.deps[n]]; dc != nil {
				cr.Externs[n] = dc
			}
		}
		crates[u.key] = cr
		binCrate = cr
	}
	for _, ms := range pmMacros {
		for _, m := range ms {
			defer m.Server.Close()
		}
	}
	r.ResolveImports()
	if err := r.Err(); err != nil {
		return err
	}
	tcx := typeck.NewTcx(r)
	entry := "main"
	if b := binCrate.Root.Lookup("__rustgo_main", ValueNS); b != nil {
		entry = "__rustgo_main"
	}
	b := binCrate.Root.Lookup(entry, ValueNS)
	if b == nil || b.Res.Def == nil || b.Res.Def.Kind != DefFn {
		return fmt.Errorf("error: `main` function not found in crate %s", binCrate.Name)
	}
	if aborted := tcx.Guard(func() { tcx.BuildImpls() }); aborted || tcx.Err() != nil {
		return tcx.Err()
	}
	g := codegen.New(tcx)
	var src string
	var genErr error
	if aborted := tcx.Guard(func() { src, genErr = g.Generate(b.Res.Def) }); aborted {
		return tcx.Err()
	}
	if genErr != nil {
		return genErr
	}
	if err := tcx.Err(); err != nil {
		return err
	}
	return compileC(opts, src)
}

// loadStd loads the std crate (embedded or from opts.StdDir).
func loadStd(opts *Options) (*ast.Crate, error) {
	if opts.StdDir != "" {
		return LoadCrateFS(os.DirFS(opts.StdDir), "lib.rs", opts.StdDir+"/")
	}
	return loadEmbeddedStd()
}
