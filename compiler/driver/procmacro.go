package driver

import (
	"crypto/sha256"
	"fmt"
	"io/fs"
	"os"
	"path/filepath"
	"sort"
	"strings"

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

// pmDecl is a proc macro exported by a proc-macro crate.
type pmDecl struct {
	kind    expand.ProcMacroKind
	name    string
	fn      string
	helpers []string
}

// scanProcMacros finds #[proc_macro], #[proc_macro_derive] and
// #[proc_macro_attribute] functions at the crate root.
func scanProcMacros(c *ast.Crate) []pmDecl {
	var out []pmDecl
	for _, it := range c.Items {
		if _, ok := it.Kind.(*ast.FnItem); !ok {
			continue
		}
		for _, a := range it.Attrs {
			switch a.Path {
			case "proc_macro":
				out = append(out, pmDecl{kind: expand.PMBang, name: it.Name.Name, fn: it.Name.Name})
			case "proc_macro_attribute":
				out = append(out, pmDecl{kind: expand.PMAttr, name: it.Name.Name, fn: it.Name.Name})
			case "proc_macro_derive":
				if len(a.Tokens) == 1 && a.Tokens[0].Delim == '(' {
					inner := a.Tokens[0].Inner
					if len(inner) == 0 {
						continue
					}
					d := pmDecl{kind: expand.PMDerive, name: inner[0].Tok.Text, fn: it.Name.Name}
					for i, tt := range inner {
						if tt.Tok.Text == "attributes" && i+1 < len(inner) && inner[i+1].Delim == '(' {
							for _, h := range inner[i+1].Inner {
								if h.Tok.Kind == lexer.Ident {
									d.helpers = append(d.helpers, h.Tok.Text)
								}
							}
						}
					}
					out = append(out, d)
				}
			}
		}
	}
	return out
}

func loadProcMacroCrate() (*ast.Crate, error) {
	sub, _ := fs.Sub(stdlib.ProcMacroFS, "proc_macro")
	return LoadCrateFS(sub, "lib.rs", "<proc_macro>/")
}

// buildProcMacroServer compiles the macro server for proc-macro crate pm
// and returns the macros it provides.
func buildProcMacroServer(opts *Options, pm *crateUnit, all map[string]*crateUnit, order []*crateUnit, _ map[string]map[string]*expand.ProcMacro) (map[string]*expand.ProcMacro, error) {
	// dependency closure of pm, in build order
	need := map[string]bool{}
	var mark func(u *crateUnit)
	mark = func(u *crateUnit) {
		if need[u.key] {
			return
		}
		need[u.key] = true
		for _, k := range u.deps {
			if du := all[k]; du != nil {
				mark(du)
			}
		}
	}
	mark(pm)
	exp := expand.New()
	stdCrate, err := loadStd(opts)
	if err != nil {
		return nil, err
	}
	if err := exp.ExpandCrate(stdCrate, "std"); err != nil {
		return nil, err
	}
	pmc, err := loadProcMacroCrate()
	if err != nil {
		return nil, err
	}
	if err := exp.ExpandCrate(pmc, "proc_macro"); err != nil {
		return nil, fmt.Errorf("proc_macro crate: %v", err)
	}
	r := resolve.New()
	r.IsBuiltinMacro = expand.IsBuiltinMacro
	r.IsCrateMacro = exp.IsCrateMacro
	r.AddCrate("std", stdCrate)
	pmCrate := r.AddCrate("proc_macro", pmc)
	pmCrate.Externs = map[string]*Crate{}
	crates := map[string]*Crate{}
	var decls []pmDecl
	for _, u := range order {
		if !need[u.key] {
			continue
		}
		if u.procMac && u != pm {
			continue
		}
		c, err := loadUnitAST(u)
		if err != nil {
			return nil, err
		}
		if u == pm {
			decls = scanProcMacros(c)
		}
		exp.Config = &expand.CrateConfig{Features: u.features, Cfgs: u.cfgs, Env: cargoEnv(u)}
		exp.Externs = u.deps
		if err := exp.ExpandCrate(c, u.key); err != nil {
			return nil, fmt.Errorf("crate %s: %v", u.name, err)
		}
		exp.Config = nil
		exp.Externs = nil
		cr := r.AddCrate(u.key, c)
		cr.Edition = u.edition
		cr.Externs = map[string]*Crate{"proc_macro": pmCrate}
		for n, k := range u.deps {
			if dc := crates[k]; dc != nil {
				cr.Externs[n] = dc
			}
		}
		crates[u.key] = cr
	}
	// generated server main
	var b strings.Builder
	b.WriteString("fn main() { ::proc_macro::__server::run(&[\n")
	for _, d := range decls {
		switch d.kind {
		case expand.PMDerive:
			fmt.Fprintf(&b, "  ::proc_macro::__server::Macro::Derive(%q, ::%s::%s as fn(::proc_macro::TokenStream) -> ::proc_macro::TokenStream),\n", d.fn, pm.name, d.fn)
		case expand.PMAttr:
			fmt.Fprintf(&b, "  ::proc_macro::__server::Macro::Attr(%q, ::%s::%s as fn(::proc_macro::TokenStream, ::proc_macro::TokenStream) -> ::proc_macro::TokenStream),\n", d.name, pm.name, d.fn)
		case expand.PMBang:
			fmt.Fprintf(&b, "  ::proc_macro::__server::Macro::Bang(%q, ::%s::%s as fn(::proc_macro::TokenStream) -> ::proc_macro::TokenStream),\n", d.name, pm.name, d.fn)
		}
	}
	b.WriteString("]); }\n")
	src := b.String()
	mc, err := parser.ParseCrate(lexer.AddFile("<proc-macro-server>", src), src)
	if err != nil {
		return nil, err
	}
	if err := exp.ExpandCrate(mc, "__pm_server"); err != nil {
		return nil, err
	}
	main := r.AddCrate("__pm_server", mc)
	main.Externs = map[string]*Crate{"proc_macro": pmCrate, pm.name: crates[pm.key]}
	r.ResolveImports()
	if err := r.Err(); err != nil {
		return nil, err
	}
	tcx := typeck.NewTcx(r)
	bnd := main.Root.Lookup("main", ValueNS)
	if aborted := tcx.Guard(func() { tcx.BuildImpls() }); aborted || tcx.Err() != nil {
		return nil, tcx.Err()
	}
	g := codegen.New(tcx)
	var prog *codegen.Program
	var genErr error
	if aborted := tcx.Guard(func() { prog, genErr = g.GenerateProgram(bnd.Res.Def) }); aborted {
		return nil, tcx.Err()
	}
	if genErr != nil {
		return nil, genErr
	}
	if err := tcx.Err(); err != nil {
		return nil, err
	}
	sum := sha256.Sum256([]byte(prog.Source()))
	dir := filepath.Join(os.TempDir(), "rustgo-proc-macros")
	os.MkdirAll(dir, 0o755)
	exe := filepath.Join(dir, fmt.Sprintf("%s-%x", pm.key, sum[:8]))
	if _, err := os.Stat(exe); err != nil {
		popts := *opts
		popts.Output = exe
		popts.EmitC = ""
		if err := compileC(&popts, prog); err != nil {
			return nil, fmt.Errorf("building proc-macro server for %s: %v", pm.name, err)
		}
	}
	server := &expand.PMServer{Path: exe}
	out := map[string]*expand.ProcMacro{}
	for _, d := range decls {
		out[d.name] = &expand.ProcMacro{Kind: d.kind, Name: d.name, Fn: d.fn, Helpers: d.helpers, Server: server}
	}
	return out, nil
}

// loadUnitAST parses a crate unit's sources.
func loadUnitAST(u *crateUnit) (*ast.Crate, error) {
	dir := filepath.Dir(u.src)
	prefix := dir + "/"
	if workspaceRoot != "" {
		if rel, err := filepath.Rel(workspaceRoot, dir); err == nil && !strings.HasPrefix(rel, "..") {
			prefix = rel + "/"
			if rel == "." {
				prefix = ""
			}
		}
	}
	c, err := LoadCrateFS(os.DirFS(dir), filepath.Base(u.src), prefix)
	if err != nil {
		return nil, fmt.Errorf("crate %s: %v", u.name, err)
	}
	return c, nil
}

func sortedKeys(m map[string]string) []string {
	ks := make([]string, 0, len(m))
	for k := range m {
		ks = append(ks, k)
	}
	sort.Strings(ks)
	return ks
}
