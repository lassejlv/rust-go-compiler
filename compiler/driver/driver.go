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
	"strings"
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
	CheckAll bool // type-check every non-generic function in all crates
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
		if hasCfg {
			// probably configured out (e.g. #[cfg(doc)] #[path = "x.md"]);
			// the expander strips it, or reports the unloaded module
			return child, nil
		}
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
	if err := exp.ExpandCrate(userCrate, "main"); err != nil {
		return nil, nil, err
	}
	t2 := time.Now()
	r := resolve.New()
	r.IsBuiltinMacro = expand.IsBuiltinMacro
	r.IsCrateMacro = exp.IsCrateMacro
	r.AddCrate("std", stdCrate)
	user := r.AddCrate("main", userCrate)
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
		src := `fn __rustgo_main() { match main() { ::std::result::Result::Ok(_) => {}, ::std::result::Result::Err(e) => { eprintln!("Error: {:?}", e); ::std::process::exit(1) } } }`
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
	tcx, mainDef, err := Frontend(opts)
	if err != nil {
		return "", err
	}
	g := codegen.New(tcx)
	var src string
	var genErr error
	aborted := tcx.Guard(func() {
		src, genErr = g.Generate(mainDef)
	})
	if aborted {
		return "", tcx.Err()
	}
	if genErr != nil {
		return "", genErr
	}
	if err := tcx.Err(); err != nil {
		return "", err
	}
	return src, nil
}

// Compile compiles opts.Input to an executable at opts.Output.
func Compile(opts *Options) error {
	src, err := GenerateC(opts)
	if err != nil {
		return err
	}
	return compileC(opts, src)
}

// compileC writes the C source and invokes the C compiler.
func compileC(opts *Options, src string) error {
	t1 := time.Now()
	if opts.EmitC != "" {
		if err := os.WriteFile(opts.EmitC, []byte(src), 0o644); err != nil {
			return err
		}
	}
	if opts.Output == "" {
		return nil
	}
	tmp, err := os.CreateTemp("", "rustgo-*.c")
	if err != nil {
		return err
	}
	cpath := tmp.Name()
	tmp.WriteString(src)
	tmp.Close()
	if !opts.KeepC {
		defer os.Remove(cpath)
	} else {
		fmt.Fprintf(os.Stderr, "C source kept at %s\n", cpath)
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
	}
	args := []string{"-O" + ol, "-w", "-fwrapv", "-fno-strict-aliasing", "-o", opts.Output, cpath, "-lm", "-lpthread"}
	args = append(args, opts.LinkArgs...)
	cmd := exec.Command(cc, args...)
	out, err := cmd.CombinedOutput()
	if err != nil {
		return fmt.Errorf("C compiler failed: %v\n%s", err, truncate(string(out), 4000))
	}
	if opts.Verbose {
		fmt.Fprintf(os.Stderr, "cc %v\n", time.Since(t1))
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
