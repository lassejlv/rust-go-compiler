// Package driver wires the compiler passes together (mirrors rustc_driver /
// rustc_interface): parse -> expand -> resolve -> typeck -> codegen (C) ->
// invoke the system C compiler.
package driver

import (
	"fmt"
	"io/fs"
	"os"
	"os/exec"
	"path"
	"path/filepath"
	"runtime"
	"strconv"
	"strings"
	"sync"
	"time"

	"rustgo/compiler/ast"
	"rustgo/compiler/codegen"
	"rustgo/compiler/expand"
	"rustgo/compiler/lexer"
	. "rustgo/compiler/middle"
	"rustgo/compiler/parser"
	"rustgo/compiler/resolve"
	"rustgo/compiler/stdlib"
	"rustgo/compiler/typeck"
)

// Options configure a compilation.
type Options struct {
	Input    string
	Output   string
	EmitC    string // if set, write the generated C here
	StdDir   string // override the embedded std with a directory
	CC       string
	OptLevel string
	Verbose  bool
	KeepC    bool
	CheckAll bool   // type-check every non-generic function in all crates
	Release  bool   // optimize (gcc -O2) and disable integer overflow checks
	Edition  string // "2015", "2018", "2021" (default) or "2024"
	Check    bool   // analysis only (like cargo check): no C compilation
	Test     bool   // build the test harness (rustc --test)
	LinkArgs []string
}

// LoadCrateFS parses a crate rooted at root inside fsys, loading `mod x;`
// files (x.rs or x/mod.rs).
func LoadCrateFS(fsys fs.FS, root, displayPrefix string) (*ast.Crate, error) {
	src, err := fs.ReadFile(fsys, root)
	if err != nil {
		return nil, err
	}
	c, err := parser.ParseCrate(lexer.AddFile(displayPrefix+root, string(src)), string(src))
	if err != nil {
		return nil, err
	}
	dir := path.Dir(root)
	base := path.Base(root)
	// A non-mod.rs, non-root file's children live in a dir named after it;
	// the crate root's children live next to it.
	if err := loadMods(fsys, c.Items, dir, displayPrefix); err != nil {
		return nil, err
	}
	_ = base
	c.Dir = dir
	c.LoadMod = func(it *ast.Item, dir string) (string, error) {
		return loadMod(fsys, it, dir, displayPrefix)
	}
	return c, nil
}

func loadMods(fsys fs.FS, items []*ast.Item, dir, prefix string) error {
	for _, it := range items {
		if _, ok := it.Kind.(*ast.ModItem); !ok {
			continue
		}
		if _, err := loadMod(fsys, it, dir, prefix); err != nil {
			return err
		}
	}
	return nil
}

// loadMod loads the file of the module item it (declared in a module whose
// children live in dir) unless it is inline or already loaded, together
// with its own out-of-line children, and returns the directory of its
// children.
func loadMod(fsys fs.FS, it *ast.Item, dir, prefix string) (string, error) {
	m := it.Kind.(*ast.ModItem)
	name := it.Name.Name
	pathAttr := ""
	hasCfg := false
	for _, a := range it.Attrs {
		if a.Path == "path" && len(a.Tokens) >= 2 {
			pathAttr = strings.Trim(a.Tokens[1].Tok.Text, "\"")
		}
		if a.Path == "cfg" || a.Path == "cfg_attr" {
			hasCfg = true
		}
	}
	child := path.Join(dir, name)
	if m.Inline {
		return child, loadMods(fsys, m.Items, child, prefix)
	}
	if m.Loaded {
		return child, nil
	}
	if hasCfg {
		// Whether the module exists, and which file it lives in
		// (`#[cfg_attr(unix, path = "unix.rs")]`), depends on the crate's
		// cfg configuration: the expander loads it through Crate.LoadMod
		// once #[cfg]/#[cfg_attr] are evaluated.
		return child, nil
	}
	cands := []string{path.Join(dir, name+".rs"), path.Join(dir, name, "mod.rs")}
	if pathAttr != "" {
		// #[path]: relative to the declaring file's directory (dir for
		// mod.rs-style parents, its parent for `foo.rs` parents)
		cands = []string{path.Join(dir, pathAttr), path.Join(path.Dir(dir), pathAttr)}
	}
	var file string
	var src []byte
	for _, cand := range cands {
		if b, err := fs.ReadFile(fsys, cand); err == nil {
			file, src = cand, b
			break
		}
	}
	if file == "" {
		return "", fmt.Errorf("%s: error: file not found for module `%s` (tried %s)", it.Span.Lo, name, strings.Join(cands, ", "))
	}
	if pathAttr != "" {
		child = path.Dir(file)
	}
	c, err := parser.ParseCrate(lexer.AddFile(prefix+file, string(src)), string(src))
	if err != nil {
		return "", err
	}
	m.Items = c.Items
	it.Attrs = append(it.Attrs, c.Attrs...) // the file's inner attributes (#![cfg(..)])
	m.Loaded = true
	return child, loadMods(fsys, m.Items, child, prefix)
}

// testHarnessMark is the hygiene mark of the generated test harness's
// paths: the harness may name private test functions (like rustc's).
const testHarnessMark = 0x7ffffff0

// addTestHarness adds the generated `main` of a --test build, which runs
// the #[test] functions through std's runner, and returns its name.
func addTestHarness(c *ast.Crate, tests []expand.TestFn) (string, error) {
	var b strings.Builder
	b.WriteString("#[allow(dead_code)]\nfn __rustgo_test_main() {\n    ::std::rustgo_test::test_main_static(&[\n")
	for _, t := range tests {
		sp := "::std::rustgo_test::ShouldPanic::No"
		switch t.ShouldPanic {
		case 1:
			sp = "::std::rustgo_test::ShouldPanic::Yes"
		case 2:
			sp = fmt.Sprintf("::std::rustgo_test::ShouldPanic::YesWithMessage(%q)", t.Expected)
		}
		im := "None"
		if t.IgnoreMsg != "" {
			im = fmt.Sprintf("Some(%q)", t.IgnoreMsg)
		}
		fmt.Fprintf(&b, "        ::std::rustgo_test::TestDesc { name: %q, ignore: %v, ignore_message: %s, should_panic: %s, file: %q, line: %d, col: %d, func: || ::std::rustgo_test::assert_test_result(crate::%s()) },\n",
			strings.Join(t.Path, "::"), t.Ignore, im, sp, t.File, t.Line, t.Col, strings.Join(t.Path, "::"))
	}
	b.WriteString("    ])\n}\n")
	src := b.String()
	hc, err := parser.ParseCrate(lexer.AddFile("<test-harness>", src), src)
	if err != nil {
		return "", fmt.Errorf("test harness: %v", err)
	}
	ast.Walk(hc, func(n any) bool {
		if pe, ok := n.(*ast.PathExpr); ok && pe.Path != nil && len(pe.Path.Segments) > 0 && pe.Path.Segments[0].Ident.Name == "crate" {
			for _, seg := range pe.Path.Segments {
				seg.Ident.Hyg = testHarnessMark
			}
		}
		return true
	})
	c.Items = append(c.Items, hc.Items...)
	return "__rustgo_test_main", nil
}

// crateNameOf derives the crate name rustc uses for a source file.
func crateNameOf(input string) string {
	n := strings.TrimSuffix(filepath.Base(input), ".rs")
	return strings.ReplaceAll(n, "-", "_")
}

// Frontend runs everything up to (and including) typeck setup and returns
// the type context and the user's main function.
func Frontend(opts *Options) (*typeck.Tcx, *Def, error) {
	t0 := time.Now()
	// std first (parsed and expanded, or from the cache), then the
	// user's crate: node IDs and file indices do not depend on whether
	// std came from the cache
	exp := expand.New()
	stdCrate, err := loadExpandedStd(opts, exp)
	if err != nil {
		return nil, nil, err
	}
	t1 := time.Now()
	abs, _ := filepath.Abs(opts.Input)
	dir := filepath.Dir(abs)
	// the user's files (and the code its macros create) get its edition
	lexer.CurEdition = opts.Edition
	defer func() { lexer.CurEdition = "" }()
	userCrate, err := LoadCrateFS(os.DirFS(dir), filepath.Base(abs), relPrefix(opts.Input))
	if err != nil {
		return nil, nil, err
	}
	entry := addMainWrapper(userCrate)
	exp.DisplayName = crateNameOf(opts.Input)
	if opts.Test {
		exp.TestHarness = true
		exp.Config = &expand.CrateConfig{Cfgs: []string{"test"}}
	}
	if err := exp.ExpandCrate(userCrate, "main"); err != nil {
		return nil, nil, err
	}
	if opts.Test {
		exp.TestHarness = false
		exp.Config = nil
		e, err := addTestHarness(userCrate, exp.Tests)
		if err != nil {
			return nil, nil, err
		}
		entry = e
	}
	entry = addGlobalAllocInit(userCrate, entry)
	t2 := time.Now()
	r := resolve.New()
	r.IsBuiltinMacro = expand.IsBuiltinMacro
	r.IsCrateMacro = exp.IsCrateMacro
	r.AddCrate("std", stdCrate)
	user := r.AddCrate("main", userCrate)
	user.Edition = opts.Edition
	user.Local = true
	user.DisplayName = crateNameOf(opts.Input)
	r.ResolveImports()
	if err := r.Err(); err != nil {
		return nil, nil, err
	}
	tcx := typeck.NewTcx(r)
	var mainDef *Def
	if b := user.Root.Lookup(entry, ValueNS); b != nil && b.Res.Def != nil && b.Res.Def.Kind == DefFn {
		mainDef = b.Res.Def
	}
	if mainDef == nil {
		return nil, nil, fmt.Errorf("error: `main` function not found in crate")
	}
	if !opts.Test {
		if err := checkMainSig(user); err != nil {
			return nil, nil, err
		}
	}
	if aborted := tcx.Guard(func() { tcx.BuildImpls() }); aborted || tcx.Err() != nil {
		return nil, nil, tcx.Err()
	}
	if opts.Verbose {
		fmt.Fprintf(os.Stderr, "std %v, parse+expand %v, resolve %v\n", t1.Sub(t0), t2.Sub(t1), time.Since(t2))
	}
	return tcx, mainDef, nil
}

// addMainWrapper adds an entry point that reports `Err` from a main
// returning Result (like std's Termination impl). Returns the entry name.
func addMainWrapper(c *ast.Crate) string {
	if e := addNoMainEntry(c); e != "" {
		return e
	}
	for _, it := range c.Items {
		fn, ok := it.Kind.(*ast.FnItem)
		if !ok || it.Name.Name != "main" || fn.Decl.Output == nil {
			continue
		}
		if tt, ok := fn.Decl.Output.(*ast.TupleType); ok && len(tt.Elems) == 0 {
			return "main"
		}
		// via Termination, so it also fits whichever `main` survives
		// cfg-stripping (several cfg'd mains with different return types)
		src := `fn __rustgo_main() { let code = ::std::process::Termination::report(main()); if code.to_i32() != 0 { ::std::process::exit(code.to_i32()) } }`
		wc, err := parser.ParseCrate(lexer.AddFile("<main-wrapper>", src), src)
		if err != nil {
			return "main"
		}
		c.Items = append(c.Items, wc.Items...)
		return "__rustgo_main"
	}
	return "main"
}

// addGlobalAllocInit wraps the entry point to install the crate's
// `#[global_allocator]` static (if any) before anything else runs. Returns
// the new entry name.
func addGlobalAllocInit(c *ast.Crate, entry string) string {
	var path []string
	var find func(items []*ast.Item, prefix []string) bool
	find = func(items []*ast.Item, prefix []string) bool {
		for _, it := range items {
			switch k := it.Kind.(type) {
			case *ast.StaticItem:
				for _, a := range it.Attrs {
					if a.Path == "global_allocator" {
						path = append(append([]string{}, prefix...), it.Name.Name)
						return true
					}
				}
			case *ast.ModItem:
				if find(k.Items, append(append([]string{}, prefix...), it.Name.Name)) {
					return true
				}
			}
		}
		return false
	}
	if !find(c.Items, nil) {
		return entry
	}
	src := fmt.Sprintf("fn __rustgo_ga_main() { ::std::alloc::__rg_register_global_alloc(&crate::%s); crate::%s(); }", strings.Join(path, "::"), entry)
	wc, err := parser.ParseCrate(lexer.AddFile("<global-allocator>", src), src)
	if err != nil {
		return entry
	}
	ast.Walk(wc, func(n any) bool {
		if pe, ok := n.(*ast.Path); ok && len(pe.Segments) > 0 && pe.Segments[0].Ident.Name == "crate" {
			for _, seg := range pe.Segments {
				seg.Ident.Hyg = testHarnessMark
			}
		}
		return true
	})
	c.Items = append(c.Items, wc.Items...)
	return "__rustgo_ga_main"
}

func loadEmbeddedStd() (*ast.Crate, error) {
	sub, _ := fs.Sub(stdlib.FS, "lib")
	return LoadCrateFS(sub, "lib.rs", "<std>/")
}

func relPrefix(input string) string {
	d := filepath.Dir(input)
	if d == "." {
		return ""
	}
	return d + "/"
}

// CheckAll type-checks every non-generic function body (in std and the
// user crate) and reports all errors.
func CheckAll(opts *Options) error {
	tcx, _, err := Frontend(opts)
	if err != nil {
		return err
	}
	tcx.MaxErrors = 1000
	tcx.Guard(func() {
		for _, d := range tcx.R.Defs {
			if d.Kind != DefFn || d.Fn == nil || d.Fn.Ast.Body == nil {
				continue
			}
			if tcx.Generics(d).Count() != 0 {
				continue
			}
			tcx.CheckInstance(d, nil)
		}
	})
	return tcx.Err()
}

// GenerateC runs the full pipeline up to C source.
func GenerateC(opts *Options) (string, error) {
	prog, err := GenerateProgram(opts)
	if err != nil {
		return "", err
	}
	return prog.Source(), nil
}

// GenerateProgram runs the full pipeline up to the generated C program.
func GenerateProgram(opts *Options) (*codegen.Program, error) {
	tcx, mainDef, err := Frontend(opts)
	if err != nil {
		return nil, err
	}
	g := codegen.New(tcx)
	var prog *codegen.Program
	var genErr error
	aborted := tcx.Guard(func() {
		prog, genErr = g.GenerateProgram(mainDef)
	})
	if aborted {
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

// Compile compiles opts.Input to an executable at opts.Output.
func Compile(opts *Options) error {
	prog, err := GenerateProgram(opts)
	if err != nil {
		return err
	}
	return compileC(opts, prog)
}

// The C program is split into several translation units compiled in
// parallel. Every unit has a fixed cost (it re-parses the common header:
// runtime, types, prototypes; about 0.13s of CPU time), and functions
// called across units can no longer be inlined. With a fixed cost h per
// unit and w for all the function definitions, n units take about
// h + w/n, so one more unit shortens the compilation while
// w/(n(n+1)) > h: in gcc -O1 terms, while n(n+1) < fnBytes/tuFixedFnBytes,
// tuFixedFnBytes being the size of the functions gcc compiles in time h.
// (A hello world gets 2 units, a 400KB program 3, large ones up to two
// units per CPU: the cost of C functions varies a lot, so smaller units
// balance better.)
const (
	tuFixedFnBytes = 48 << 10
	tuMax          = 16
)

// unitCount picks the number of translation units for a program with
// fnBytes of function definitions (RUSTGO_CC_UNITS overrides it).
func unitCount(fnBytes int) int {
	if s := os.Getenv("RUSTGO_CC_UNITS"); s != "" {
		if n, err := strconv.Atoi(s); err == nil && n >= 1 {
			return n
		}
	}
	max := 2 * runtime.NumCPU()
	if runtime.NumCPU() == 1 {
		max = 1
	}
	if max > tuMax {
		max = tuMax
	}
	n := 1
	for n < max && n*(n+1)*tuFixedFnBytes < fnBytes {
		n++
	}
	return n
}

// releaseUnitCount picks the number of translation units of a release
// build: functions called across units cannot be inlined, which costs
// rustgo's C (many small functions) about half its speed, so release builds
// stay in one unit up to releaseOneUnitBytes of function definitions (a
// single unit also takes less CPU time in total) and larger programs get
// units several times larger than debug builds.
const releaseOneUnitBytes = 4 << 20

func releaseUnitCount(fnBytes int) int {
	if fnBytes <= releaseOneUnitBytes {
		return 1
	}
	return unitCount(fnBytes / 8)
}

// debugCFlags turn off the gcc -O1 passes that cost the most compile time
// for the least speed of rustgo's C (IPA analyses and points-to, scalar
// replacement, dead stores, jump threading, loop header copying, value
// numbering, forward propagation, if-conversion, branch probabilities):
// debug builds (no --release, no -O) compile about 20% faster than with
// -O1 and still run at about -O1 speed (-O0 compiles only a little faster
// but runs several times slower: rustgo's C relies on register allocation
// and inlining of its many small functions).
var debugCFlags = []string{
	"-fno-ipa-pure-const", "-fno-ipa-reference", "-fno-ipa-modref", "-fno-tree-pta",
	"-fno-tree-sra", "-fno-tree-dse", "-fno-dse",
	"-fno-tree-dominator-opts", "-fno-tree-ch", "-fno-tree-loop-optimize",
	"-fno-tree-fre", "-fno-tree-forwprop",
	"-fno-guess-branch-probability", "-fno-if-conversion", "-fno-if-conversion2",
}

// compileC writes the C source and invokes the C compiler. Large programs
// are compiled as several translation units in parallel, then linked.
func compileC(opts *Options, prog *codegen.Program) error {
	t1 := time.Now()
	if opts.EmitC != "" {
		if err := os.WriteFile(opts.EmitC, []byte(prog.Source()), 0o644); err != nil {
			return err
		}
	}
	if opts.Output == "" || opts.Check {
		return nil
	}
	cc := opts.CC
	if cc == "" {
		cc = os.Getenv("RUSTGO_CC")
	}
	if cc == "" {
		cc = "gcc"
	}
	ol := opts.OptLevel
	debug := false
	if ol == "" {
		ol = "1"
		if opts.Release {
			ol = "2"
		} else {
			debug = true
		}
	}
	cflags := []string{"-O" + ol, "-w", "-fwrapv", "-fno-strict-aliasing", "-pipe"}
	if debug && strings.Contains(filepath.Base(cc), "gcc") { // (gcc flags)
		cflags = append(cflags, debugCFlags...)
	}
	if opts.Release {
		cflags = append(cflags, "-DRG_NO_OVERFLOW_CHECKS")
	}
	if prog.UsesAsm {
		cflags = append(cflags, "-masm=intel") // Rust asm! templates are Intel syntax
	}
	// extra flags for the C compiler (experiments)
	cflags = append(cflags, strings.Fields(os.Getenv("RUSTGO_CFLAGS"))...)
	libs := append(append([]string{"-lm", "-lpthread"}, opts.LinkArgs...), prog.LinkArgs...)
	nu := unitCount(prog.FnBytes())
	if opts.Release && os.Getenv("RUSTGO_CC_UNITS") == "" {
		nu = releaseUnitCount(prog.FnBytes())
	}
	units := prog.Units(nu)
	if len(units) == 1 {
		tmp, err := os.CreateTemp("", "rustgo-*.c")
		if err != nil {
			return err
		}
		cpath := tmp.Name()
		tmp.WriteString(units[0])
		tmp.Close()
		if !opts.KeepC {
			defer os.Remove(cpath)
		} else {
			fmt.Fprintf(os.Stderr, "C source kept at %s\n", cpath)
		}
		args := append(append(cflags, "-o", opts.Output, cpath), libs...)
		if out, err := exec.Command(cc, args...).CombinedOutput(); err != nil {
			return fmt.Errorf("C compiler failed: %v\n%s", err, truncate(string(out), 4000))
		}
		if opts.Verbose {
			fmt.Fprintf(os.Stderr, "cc %v\n", time.Since(t1))
		}
		return nil
	}
	dir, err := os.MkdirTemp("", "rustgo-*")
	if err != nil {
		return err
	}
	if !opts.KeepC {
		defer os.RemoveAll(dir)
	} else {
		fmt.Fprintf(os.Stderr, "C sources (%d translation units) kept in %s\n", len(units), dir)
	}
	objs := make([]string, len(units))
	errs := make([]error, len(units))
	var wg sync.WaitGroup
	sem := make(chan struct{}, runtime.NumCPU())
	cpaths := make([]string, len(units))
	for i, src := range units {
		cpaths[i] = filepath.Join(dir, fmt.Sprintf("u%d.c", i))
		objs[i] = filepath.Join(dir, fmt.Sprintf("u%d.o", i))
		if err := os.WriteFile(cpaths[i], []byte(src), 0o644); err != nil {
			return err
		}
	}
	cache := newCCCache(cc, cflags)
	hits := 0
	var hitMu sync.Mutex
	for i, cpath := range cpaths {
		wg.Add(1)
		go func(i int, cpath string) {
			defer wg.Done()
			if cache != nil && cache.get(units[i], objs[i]) {
				hitMu.Lock()
				hits++
				hitMu.Unlock()
				return
			}
			sem <- struct{}{}
			defer func() { <-sem }()
			args := append(append([]string{}, cflags...), "-c", "-o", objs[i], cpath)
			if out, err := exec.Command(cc, args...).CombinedOutput(); err != nil {
				errs[i] = fmt.Errorf("C compiler failed: %v\n%s", err, truncate(string(out), 4000))
			} else if cache != nil {
				cache.add(units[i], objs[i])
			}
		}(i, cpath)
	}
	wg.Wait()
	if cache != nil {
		cache.prune()
	}
	for _, err := range errs {
		if err != nil {
			return err
		}
	}
	t2 := time.Now()
	args := append(append([]string{"-o", opts.Output}, objs...), libs...)
	if out, err := exec.Command(cc, args...).CombinedOutput(); err != nil {
		return fmt.Errorf("linking failed: %v\n%s", err, truncate(string(out), 4000))
	}
	if opts.Verbose {
		fmt.Fprintf(os.Stderr, "cc %v (%d units, %d cached: compile %v, link %v)\n", time.Since(t1), len(units), hits, t2.Sub(t1), time.Since(t2))
	}
	return nil
}

func truncate(s string, n int) string {
	if len(s) > n {
		return s[:n] + "\n..."
	}
	return s
}

// loadStd loads the std crate (embedded or from opts.StdDir).
func loadStd(opts *Options) (*ast.Crate, error) {
	if opts.StdDir != "" {
		return LoadCrateFS(os.DirFS(opts.StdDir), "lib.rs", opts.StdDir+"/")
	}
	return loadEmbeddedStd()
}

// addNoMainEntry handles a `#![no_main]` crate whose entry point is a
// `#[no_mangle] extern "C" fn main([argc, argv]) -> c_int`: the function
// keeps its Rust name (the runtime provides the C `main`) and is called by a
// generated entry that exits with its result. Returns "" when the crate is
// not of that shape.
func addNoMainEntry(c *ast.Crate) string {
	noMain := false
	for _, a := range c.Attrs {
		if a.Path == "no_main" {
			noMain = true
		}
	}
	if !noMain {
		return ""
	}
	for _, it := range c.Items {
		fn, ok := it.Kind.(*ast.FnItem)
		if !ok || it.Name.Name != "main" {
			continue
		}
		var attrs []*ast.Attr
		for _, a := range it.Attrs {
			if a.Path != "no_mangle" && a.Path != "unsafe(no_mangle)" {
				attrs = append(attrs, a)
			}
		}
		it.Attrs = attrs
		call := "main()"
		if len(fn.Decl.Params) == 2 {
			call = "main(::std::process::__rg_argc() as _, ::std::process::__rg_argv() as _)"
		}
		src := "fn __rustgo_main() { let code = unsafe { " + call + " }; ::std::process::exit(code as i32) }"
		if fn.Decl.Output == nil {
			src = "fn __rustgo_main() { unsafe { " + call + " }; }"
		}
		wc, err := parser.ParseCrate(lexer.AddFile("<main-wrapper>", src), src)
		if err != nil {
			return ""
		}
		c.Items = append(c.Items, wc.Items...)
		return "__rustgo_main"
	}
	return ""
}

// checkMainSig reports a `main` function of a binary crate that takes
// parameters (E0580) or has generic parameters (E0131).
func checkMainSig(c *Crate) error {
	if c.Ast != nil {
		for _, a := range c.Ast.Attrs {
			if a.Path == "no_main" {
				return nil
			}
		}
	}
	b := c.Root.Lookup("main", ValueNS)
	if b == nil || b.Res.Def == nil || b.Res.Def.Kind != DefFn || b.Res.Def.Fn == nil || b.Res.Def.Fn.Ast == nil {
		return nil
	}
	fn := b.Res.Def.Fn.Ast
	sp := b.Res.Def.Span
	if it := b.Res.Def.Item; it != nil {
		sp = it.Name.Span
	}
	if fn.Generics != nil {
		for _, p := range fn.Generics.Params {
			if p.Kind != ast.GPLifetime {
				return fmt.Errorf("%s: error[E0131]: `main` function is not allowed to have generic parameters", sp.Lo)
			}
		}
	}
	if len(fn.Decl.Params) > 0 || fn.Decl.SelfParam != nil || fn.Unsafe || (fn.Abi != "" && fn.Abi != "Rust") {
		return fmt.Errorf("%s: error[E0580]: `main` function has wrong type", sp.Lo)
	}
	if fn.Generics != nil && len(fn.Generics.Where) > 0 {
		return fmt.Errorf("%s: error[E0646]: `main` function is not allowed to have a `where` clause", sp.Lo)
	}
	if it, ok := fn.Decl.Output.(*ast.ImplTraitType); ok {
		// (an `async fn main` returns `impl Future` after desugaring)
		for _, b := range it.Bounds {
			if b.Path != nil && len(b.Path.Segments) > 0 && b.Path.Segments[len(b.Path.Segments)-1].Ident.Name == "Future" {
				return fmt.Errorf("%s: error[E0752]: `main` function is not allowed to be `async`", sp.Lo)
			}
		}
		return fmt.Errorf("%s: error[E0277]: `main` has invalid return type", sp.Lo)
	}
	return nil
}
