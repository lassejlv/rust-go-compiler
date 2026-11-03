//go:build ignore

// gen_serial generates serial_gen.go: binary encoders/decoders for every
// AST node type declared in ast.go (used to cache the expanded standard
// library between compiler runs, see Encoder/Decoder in serial.go).
//
//	go run gen_serial.go            (rewrites serial_gen.go)
//	go run gen_serial.go -stdout    (prints it; used by TestSerialGenUpToDate)
//
// Every field of every struct is serialized, except the ones listed in
// skipFields (a decoded node has their zero value) and function-typed
// fields; fields listed in failFields make encoding fail when set (the
// cache is then not written). Adding a field to an AST type therefore only
// requires re-running this generator; the test fails until it is done.
package main

import (
	"bytes"
	"fmt"
	"go/ast"
	"go/format"
	"go/parser"
	"go/token"
	"os"
	"sort"
	"strings"
)

// skipFields are not serialized (decoded nodes have the zero value).
var skipFields = map[string]bool{
	// source tokens of items, only fed to procedural macros: the
	// serialized crates are fully expanded
	"Item.Tokens": true,
}

// failFields make encoding fail when non-zero (pointers that may alias
// other nodes of the tree).
var failFields = map[string]bool{
	"ClosureExpr.MoveParamsOf": true,
}

// markers maps each node interface to the marker method of its
// implementations.
var markers = map[string]string{
	"Type":     "typeNode",
	"ItemKind": "itemKind",
	"Stmt":     "stmtNode",
	"Expr":     "exprNode",
	"Pat":      "patNode",
}

type field struct {
	name     string // "" for embedded
	typ      ast.Expr
	embedded string
}

type gen struct {
	structs     map[string][]field
	order       []string
	named       map[string]string   // named non-struct types -> underlying ident
	impls       map[string][]string // interface -> implementing struct types
	out         bytes.Buffer
	helpers     map[string]bool
	pending     []ast.Expr
	helperBody  bytes.Buffer
	slabs       []string // struct types allocated in chunks
	slabSet     map[string]bool
	sliceAllocs [][2]string // helper name, slice type
}

// slab records that values of struct type t are allocated by the decoder
// in chunks (one allocation per slabSize values instead of one each).
func (g *gen) slab(t string) {
	if g.slabSet == nil {
		g.slabSet = map[string]bool{}
	}
	if !g.slabSet[t] {
		g.slabSet[t] = true
		g.slabs = append(g.slabs, t)
	}
}

func (g *gen) sliceAlloc(name, ts string) { g.sliceAllocs = append(g.sliceAllocs, [2]string{name, ts}) }

func main() {
	fset := token.NewFileSet()
	f, err := parser.ParseFile(fset, "ast.go", nil, 0)
	if err != nil {
		panic(err)
	}
	g := &gen{structs: map[string][]field{}, named: map[string]string{}, impls: map[string][]string{}, helpers: map[string]bool{}}
	methods := map[string]map[string]bool{} // type -> methods
	for _, d := range f.Decls {
		switch d := d.(type) {
		case *ast.GenDecl:
			for _, s := range d.Specs {
				ts, ok := s.(*ast.TypeSpec)
				if !ok {
					continue
				}
				switch t := ts.Type.(type) {
				case *ast.StructType:
					var fs []field
					for _, fl := range t.Fields.List {
						if len(fl.Names) == 0 {
							fs = append(fs, field{typ: fl.Type, embedded: fl.Type.(*ast.Ident).Name})
							continue
						}
						for _, n := range fl.Names {
							fs = append(fs, field{name: n.Name, typ: fl.Type})
						}
					}
					g.structs[ts.Name.Name] = fs
					g.order = append(g.order, ts.Name.Name)
				case *ast.Ident:
					if !ts.Assign.IsValid() {
						g.named[ts.Name.Name] = t.Name
					}
				}
			}
		case *ast.FuncDecl:
			if d.Recv == nil || len(d.Recv.List) != 1 {
				continue
			}
			rt := d.Recv.List[0].Type
			if st, ok := rt.(*ast.StarExpr); ok {
				rt = st.X
			}
			if id, ok := rt.(*ast.Ident); ok {
				if methods[id.Name] == nil {
					methods[id.Name] = map[string]bool{}
				}
				methods[id.Name][d.Name.Name] = true
			}
		}
	}
	var has func(t, m string) bool
	has = func(t, m string) bool {
		if methods[t][m] {
			return true
		}
		for _, fl := range g.structs[t] {
			if fl.embedded != "" && has(fl.embedded, m) {
				return true
			}
		}
		return false
	}
	for iface, m := range markers {
		for _, t := range g.order {
			if strings.HasSuffix(t, "Base") {
				continue
			}
			if has(t, m) {
				g.impls[iface] = append(g.impls[iface], t)
			}
		}
		if len(g.impls[iface]) == 0 {
			panic("no implementations of " + iface)
		}
	}
	g.generate()
	src, err := format.Source(g.out.Bytes())
	if err != nil {
		os.Stdout.Write(g.out.Bytes())
		panic(err)
	}
	if len(os.Args) > 1 && os.Args[1] == "-stdout" {
		os.Stdout.Write(src)
		return
	}
	if err := os.WriteFile("serial_gen.go", src, 0o644); err != nil {
		panic(err)
	}
}

func (g *gen) p(format string, args ...any) { fmt.Fprintf(&g.out, format+"\n", args...) }

func exprStr(e ast.Expr) string {
	switch e := e.(type) {
	case *ast.Ident:
		return e.Name
	case *ast.StarExpr:
		return "*" + exprStr(e.X)
	case *ast.ArrayType:
		return "[]" + exprStr(e.Elt)
	case *ast.FuncType:
		return "func"
	}
	panic(fmt.Sprintf("unsupported type expression %T", e))
}

// helperName names the encode/decode helper pair of a pointer or slice type.
func helperName(e ast.Expr) string {
	s := exprStr(e)
	s = strings.ReplaceAll(s, "[]", "Sl_")
	s = strings.ReplaceAll(s, "*", "Pt_")
	return s
}

func (g *gen) isIface(n string) bool { _, ok := markers[n]; return ok }

// encExpr returns a statement encoding the value v of type t.
func (g *gen) encStmt(t ast.Expr, v string) string {
	switch t := t.(type) {
	case *ast.Ident:
		switch n := t.Name; {
		case n == "bool":
			return fmt.Sprintf("e.b(%s)", v)
		case n == "string":
			return fmt.Sprintf("e.s(%s)", v)
		case n == "int" || n == "rune":
			return fmt.Sprintf("e.i(int64(%s))", v)
		case n == "uint32" || n == "uint64" || n == "byte":
			return fmt.Sprintf("e.u(uint64(%s))", v)
		case n == "NodeID":
			return fmt.Sprintf("e.id(%s)", v)
		case n == "Span":
			return fmt.Sprintf("e.span(%s)", v)
		case n == "Token":
			return fmt.Sprintf("e.tok(&%s)", v)
		case g.named[n] != "":
			return fmt.Sprintf("e.i(int64(%s))", v)
		case g.isIface(n):
			return fmt.Sprintf("e.enc%s(%s)", n, v)
		case g.structs[n] != nil:
			return fmt.Sprintf("e.enc%s(&%s)", n, v)
		}
		panic("unknown type " + t.Name)
	case *ast.StarExpr, *ast.ArrayType:
		g.need(t)
		return fmt.Sprintf("e.enc%s(%s)", helperName(t), v)
	}
	panic(fmt.Sprintf("unsupported field type %T", t))
}

// decStmt returns a statement decoding a value of type t into the
// addressable expression v.
func (g *gen) decStmt(t ast.Expr, v string) string {
	switch t := t.(type) {
	case *ast.Ident:
		switch n := t.Name; {
		case n == "bool":
			return fmt.Sprintf("%s = d.b()", v)
		case n == "string":
			return fmt.Sprintf("%s = d.s()", v)
		case n == "int" || n == "rune" || n == "uint32" || n == "uint64" || n == "byte":
			if n == "int" || n == "rune" {
				return fmt.Sprintf("%s = %s(d.i())", v, n)
			}
			return fmt.Sprintf("%s = %s(d.u())", v, n)
		case n == "NodeID":
			return fmt.Sprintf("%s = d.id()", v)
		case n == "Span":
			return fmt.Sprintf("%s = d.span()", v)
		case n == "Token":
			return fmt.Sprintf("d.tok(&%s)", v)
		case g.named[n] != "":
			return fmt.Sprintf("%s = %s(d.i())", v, n)
		case g.isIface(n):
			return fmt.Sprintf("%s = d.dec%s()", v, n)
		case g.structs[n] != nil:
			return fmt.Sprintf("d.dec%s(&%s)", n, v)
		}
		panic("unknown type " + t.Name)
	case *ast.StarExpr, *ast.ArrayType:
		g.need(t)
		return fmt.Sprintf("%s = d.dec%s()", v, helperName(t))
	}
	panic(fmt.Sprintf("unsupported field type %T", t))
}

func (g *gen) need(t ast.Expr) {
	n := helperName(t)
	if g.helpers[n] {
		return
	}
	g.helpers[n] = true
	g.pending = append(g.pending, t)
}

func (g *gen) emitHelper(t ast.Expr) {
	n := helperName(t)
	ts := exprStr(t)
	switch t := t.(type) {
	case *ast.StarExpr:
		el := t.X.(*ast.Ident).Name
		if g.structs[el] == nil {
			panic("pointer to non-struct " + el)
		}
		g.p("func (e *Encoder) enc%s(x %s) {", n, ts)
		g.p("if x == nil { e.u(0); return }")
		g.p("e.u(1); e.mark(x); e.enc%s(x)", el)
		g.p("}")
		g.p("func (d *Decoder) dec%s() %s {", n, ts)
		g.p("if d.u() == 0 { return nil }")
		g.p("x := d.new%s(); d.dec%s(x); return x", el, el)
		g.slab(el)
		g.p("}")
	case *ast.ArrayType:
		g.p("func (e *Encoder) enc%s(x %s) {", n, ts)
		g.p("if x == nil { e.u(0); return }")
		g.p("e.u(uint64(len(x)) + 1)")
		g.p("for i := range x { %s }", g.encStmt(t.Elt, "x[i]"))
		g.p("}")
		g.p("func (d *Decoder) dec%s() %s {", n, ts)
		g.p("n := d.u()")
		g.p("if n == 0 { return nil }")
		g.p("x := d.make%s(d.length(n-1))", n)
		g.sliceAlloc(n, ts)
		g.p("for i := range x { %s }", g.decStmt(t.Elt, "x[i]"))
		g.p("return x")
		g.p("}")
	}
}

func (g *gen) generate() {
	g.p("// Code generated by gen_serial.go; DO NOT EDIT.")
	g.p("")
	g.p("package ast")
	g.p("")
	// interfaces
	ifaces := make([]string, 0, len(markers))
	for k := range markers {
		ifaces = append(ifaces, k)
	}
	sort.Strings(ifaces)
	for _, iface := range ifaces {
		g.p("func (e *Encoder) enc%s(x %s) {", iface, iface)
		g.p("switch v := x.(type) {")
		g.p("case nil: e.u(0)")
		for i, t := range g.impls[iface] {
			g.p("case *%s: e.u(%d); e.mark(v); e.enc%s(v)", t, i+1, t)
		}
		g.p("default: e.fail(\"unknown %s node %%T\", x)", iface)
		g.p("}")
		g.p("}")
		g.p("func (d *Decoder) dec%s() %s {", iface, iface)
		g.p("switch d.u() {")
		g.p("case 0: return nil")
		for i, t := range g.impls[iface] {
			g.p("case %d: x := d.new%s(); d.dec%s(x); return x", i+1, t, t)
			g.slab(t)
		}
		g.p("}")
		g.p("panic(errCorrupt)")
		g.p("}")
	}
	// structs
	for _, s := range g.order {
		if s == "Crate" {
			continue // driven by hand (EncodeCrate)
		}
		var enc, dec []string
		for _, f := range g.structs[s] {
			if f.embedded != "" {
				enc = append(enc, fmt.Sprintf("e.enc%s(&x.%s)", f.embedded, f.embedded))
				dec = append(dec, fmt.Sprintf("d.dec%s(&x.%s)", f.embedded, f.embedded))
				continue
			}
			key := s + "." + f.name
			if _, isFunc := f.typ.(*ast.FuncType); isFunc || skipFields[key] {
				continue
			}
			if failFields[key] {
				enc = append(enc, fmt.Sprintf("if x.%s != nil { e.fail(\"cannot serialize %s\") }", f.name, key))
				continue
			}
			if f.name == "Hyg" {
				enc = append(enc, fmt.Sprintf("e.hyg(x.%s)", f.name))
				dec = append(dec, fmt.Sprintf("x.%s = d.hyg()", f.name))
				continue
			}
			enc = append(enc, g.encStmt(f.typ, "x."+f.name))
			dec = append(dec, g.decStmt(f.typ, "x."+f.name))
		}
		g.p("func (e *Encoder) enc%s(x *%s) {", s, s)
		for _, l := range enc {
			g.p("%s", l)
		}
		g.p("}")
		g.p("func (d *Decoder) dec%s(x *%s) {", s, s)
		for _, l := range dec {
			g.p("%s", l)
		}
		g.p("}")
	}
	// Crate fields serialized by EncodeCrate
	g.need(&ast.ArrayType{Elt: &ast.StarExpr{X: ast.NewIdent("Attr")}})
	g.need(&ast.ArrayType{Elt: &ast.StarExpr{X: ast.NewIdent("Item")}})
	g.need(&ast.ArrayType{Elt: ast.NewIdent("TokenTree")})
	for len(g.pending) > 0 {
		t := g.pending[0]
		g.pending = g.pending[1:]
		g.emitHelper(t)
	}
	// chunked allocation
	g.p("type decAlloc struct {")
	for _, t := range g.slabs {
		g.p("slab%s []%s", t, t)
	}
	for _, sa := range g.sliceAllocs {
		g.p("chunk%s %s", sa[0], sa[1])
	}
	g.p("}")
	for _, t := range g.slabs {
		g.p("func (d *Decoder) new%s() *%s {", t, t)
		g.p("if len(d.slab%s) == 0 { d.slab%s = make([]%s, slabSize) }", t, t, t)
		g.p("x := &d.slab%s[0]; d.slab%s = d.slab%s[1:]; return x", t, t, t)
		g.p("}")
	}
	for _, sa := range g.sliceAllocs {
		n, ts := sa[0], sa[1]
		g.p("func (d *Decoder) make%s(n int) %s {", n, ts)
		g.p("if n == 0 || n > chunkSize/4 { return make(%s, n) } // (never nil)", ts)
		g.p("if len(d.chunk%s) < n { d.chunk%s = make(%s, chunkSize) }", n, n, ts)
		g.p("x := d.chunk%s[:n:n]; d.chunk%s = d.chunk%s[n:]; return x", n, n, n)
		g.p("}")
	}
}
