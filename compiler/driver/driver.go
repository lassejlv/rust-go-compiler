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

// crateNameOf derives the crate name rustc uses for a source file.
func crateNameOf(input string) string {
	n := strings.TrimSuffix(filepath.Base(input), ".rs")
	return strings.ReplaceAll(n, "-", "_")
}

// Frontend runs everything up to (and including) typeck setup and returns
// the type context and the user's main function.
func Frontend(opts *Options) (*typeck.Tcx, *Def, error) {
	t0 := time.Now()
	stdCrate, err := loadStd(opts)
	if err != nil {
		return nil, nil, fmt.Errorf("while loading std: %v", err)
	}
	abs, _ := filepath.Abs(opts.Input)
	dir := filepath.Dir(abs)
	userCrate, err := LoadCrateFS(os.DirFS(dir), filepath.Base(abs), relPrefix(opts.Input))
	if err != nil {
		return nil, nil, err
	}
	entry := addMainWrapper(userCrate)
	t1 := time.Now()
	exp := expand.New()
	if err := exp.ExpandCrate(stdCrate, "std"); err != nil {
		return nil, nil, fmt.Errorf("while expanding std: %v", err)
	}
	exp.DisplayName = crateNameOf(opts.Input)
	if err := exp.ExpandCrate(userCrate, "main"); err != nil {
		return nil, nil, err
	}
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
	if aborted := tcx.Guard(func() { tcx.BuildImpls() }); aborted || tcx.Err() != nil {
		return nil, nil, tcx.Err()
	}
	if opts.Verbose {
		fmt.Fprintf(os.Stderr, "parse %v, expand %v, resolve %v\n", t1.Sub(t0), t2.Sub(t1), time.Since(t2))
	}
	return tcx, mainDef, nil
}

// addMainWrapper adds an entry point that reports `Err` from a main
// returning Result (like std's Termination impl). Returns the entry name.
func addMainWrapper(c *ast.Crate) string {
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

// Splitting the C program into several translation units compiled in
// parallel pays off only for large programs: every unit re-parses the
// common header (runtime, types, prototypes), and functions called across
// units can no longer be inlined. Up to two units per CPU are made (the
// cost of C functions varies a lot, so smaller units balance better), each
// with at least tuMinFnBytes of function definitions.
const (
	tuMinFnBytes = 1 << 20
	tuMax        = 16
)

// unitCount picks the number of translation units for a program with
// fnBytes of function definitions (RUSTGO_CC_UNITS overrides it).
func unitCount(fnBytes int) int {
	if s := os.Getenv("RUSTGO_CC_UNITS"); s != "" {
		if n, err := strconv.Atoi(s); err == nil && n >= 1 {
			return n
		}
	}
	n := 2 * runtime.NumCPU()
	if n > tuMax {
		n = tuMax
	}
	if m := fnBytes / tuMinFnBytes; m < n {
		n = m
	}
	if n < 1 {
		n = 1
	}
	return n
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
	if ol == "" {
		ol = "1"
		if opts.Release {
			ol = "2"
		}
	}
	cflags := []string{"-O" + ol, "-w", "-fwrapv", "-fno-strict-aliasing", "-pipe"}
	if opts.Release {
		cflags = append(cflags, "-DRG_NO_OVERFLOW_CHECKS")
	}
	if prog.UsesAsm {
		cflags = append(cflags, "-masm=intel") // Rust asm! templates are Intel syntax
	}
	libs := append([]string{"-lm", "-lpthread"}, opts.LinkArgs...)
	units := prog.Units(unitCount(prog.FnBytes()))
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
	for i, cpath := range cpaths {
		wg.Add(1)
		go func(i int, cpath string) {
			defer wg.Done()
			sem <- struct{}{}
			defer func() { <-sem }()
			args := append(append([]string{}, cflags...), "-c", "-o", objs[i], cpath)
			if out, err := exec.Command(cc, args...).CombinedOutput(); err != nil {
				errs[i] = fmt.Errorf("C compiler failed: %v\n%s", err, truncate(string(out), 4000))
			}
		}(i, cpath)
	}
	wg.Wait()
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
		fmt.Fprintf(os.Stderr, "cc %v (%d units: compile %v, link %v)\n", time.Since(t1), len(units), t2.Sub(t1), time.Since(t2))
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
