package codegen

import (
	_ "embed"
	"fmt"
	"sort"
	"strings"

	"rustgo/compiler/ast"
	"rustgo/compiler/lexer"
	. "rustgo/compiler/middle"
	"rustgo/compiler/typeck"
)

//go:embed runtime.h
var runtimeH string

type instance struct {
	def  *Def
	args []Ty
}

// Gen generates a C translation unit.
type Gen struct {
	tcx *typeck.Tcx

	fwd     strings.Builder // typedefs
	defs    strings.Builder // struct definitions
	protos  strings.Builder // prototypes
	vtdefs  strings.Builder // vtable definitions
	statics strings.Builder
	fns     chunkBuf // function bodies (one chunk per function)

	types        map[Ty]*typeInfo
	typeN        int
	pendingTypes []Ty

	fnNames      map[string]string
	fnQueue      []instance
	fnN          int
	closureFns   map[Ty]string
	closureQueue []Ty

	dropFns        map[Ty]string
	dropQueue      []Ty
	needsDropCache map[Ty]bool
	dropImplCache  map[Ty]*instance

	vtables    map[string]string
	nicheCache map[Ty]*nicheLayout
	statics_   map[*Def]string
	staticInit []string
	strN       int

	Errors []string
}

// New creates a generator.
func New(tcx *typeck.Tcx) *Gen {
	return &Gen{
		tcx:            tcx,
		types:          map[Ty]*typeInfo{},
		fnNames:        map[string]string{},
		closureFns:     map[Ty]string{},
		dropFns:        map[Ty]string{},
		needsDropCache: map[Ty]bool{},
		dropImplCache:  map[Ty]*instance{},
		vtables:        map[string]string{},
		nicheCache:     map[Ty]*nicheLayout{},
		statics_:       map[*Def]string{},
	}
}

func (g *Gen) errorf(sp lexer.Span, format string, args ...any) {
	g.Errors = append(g.Errors, fmt.Sprintf("%s: error: %s", sp.Lo, fmt.Sprintf(format, args...)))
}

// fnName returns the C name of a function instance, queueing it.
func (g *Gen) fnName(d *Def, args []Ty) string {
	key := typeck.InstKey(d, args)
	if n, ok := g.fnNames[key]; ok {
		return n
	}
	g.fnN++
	name := fmt.Sprintf("f%d_%s", g.fnN, cIdent(d.Name))
	if d.Intrinsic == "" && d.Fn != nil && d.Fn.Extern {
		// foreign fn, declared with an asm label; the C name is unique per
		// declaration (two modules may declare the same symbol differently)
		name = fmt.Sprintf("rgx%d_%s", g.fnN, cIdent(d.Name))
	}
	g.fnNames[key] = name
	g.fnQueue = append(g.fnQueue, instance{def: d, args: args})
	return name
}

func cIdent(s string) string {
	var b strings.Builder
	for _, r := range s {
		if r == '_' || (r >= 'a' && r <= 'z') || (r >= 'A' && r <= 'Z') || (r >= '0' && r <= '9') {
			b.WriteRune(r)
		} else {
			b.WriteByte('_')
		}
	}
	if b.Len() > 24 {
		return b.String()[:24]
	}
	return b.String()
}

// Generate produces the C source for a program whose entry is mainDef.
func (g *Gen) Generate(mainDef *Def) (string, error) {
	p, err := g.GenerateProgram(mainDef)
	if err != nil {
		return "", err
	}
	return p.Source(), nil
}

// GenerateProgram produces the C program whose entry is mainDef, which can
// be rendered as one translation unit or split into several (see Units).
func (g *Gen) GenerateProgram(mainDef *Def) (prog *Program, err error) {
	defer func() {
		if r := recover(); r != nil {
			if ce, ok := r.(cgError); ok {
				err = fmt.Errorf("%s", ce.msg)
				return
			}
			panic(r)
		}
	}()
	mainName := g.fnName(mainDef, nil)
	ctors := g.initArrayCtors()
	g.drain()
	g.staticInit = append(g.staticInit, ctors...) // after every static's init
	if len(g.tcx.Errors) > 0 {
		return nil, g.tcx.Err()
	}
	if len(g.Errors) > 0 {
		return nil, fmt.Errorf("%s", strings.Join(g.Errors, "\n"))
	}
	var m strings.Builder
	m.WriteString("int main(int argc, char** argv) {\n  rg_init(argc, argv);\n")
	for _, s := range g.staticInit {
		m.WriteString("  " + s + "\n")
	}
	fmt.Fprintf(&m, "  %s();\n", mainName)
	m.WriteString("  rg_flush_stdout();\n  return 0;\n}\n")
	return &Program{
		types:  g.fwd.String() + g.defs.String(),
		protos: g.protos.String(),
		data:   g.statics.String() + g.vtdefs.String(),
		fns:    g.fns.String(),
		fnEnds: g.fns.ends,
		main:   m.String(),
	}, nil
}

type cgError struct{ msg string }

func (g *Gen) fatalf(format string, args ...any) {
	panic(cgError{fmt.Sprintf(format, args...)})
}

// drain generates all queued functions, closures, drop glue and types.
func (g *Gen) drain() {
	for {
		progressed := false
		for len(g.fnQueue) > 0 {
			inst := g.fnQueue[0]
			g.fnQueue = g.fnQueue[1:]
			g.genFunction(inst)
			progressed = true
		}
		for len(g.closureQueue) > 0 {
			t := g.closureQueue[0]
			g.closureQueue = g.closureQueue[1:]
			g.genClosure(t)
			progressed = true
		}
		for len(g.dropQueue) > 0 {
			t := g.dropQueue[0]
			g.dropQueue = g.dropQueue[1:]
			g.emitDropGlue(t)
			progressed = true
		}
		for len(g.pendingTypes) > 0 {
			t := g.pendingTypes[0]
			g.pendingTypes = g.pendingTypes[1:]
			g.defineType(t)
			progressed = true
		}
		if !progressed {
			return
		}
	}
}

// genFunction emits the C definition of a function instance.
func (g *Gen) genFunction(inst instance) {
	d := inst.def
	name := g.fnNames[typeck.InstKey(d, inst.args)]
	switch {
	case d.Kind == DefStruct || d.Kind == DefVariant:
		g.genCtorFn(name, d, inst.args)
		return
	case d.Kind == DefConst || d.Kind == DefStatic:
		g.genConstFn(name, d, inst.args)
		return
	}
	if d.Fn.Extern && d.Intrinsic == "" {
		g.genForeignDecl(name, d, inst.args)
		return
	}
	if d.Intrinsic != "" {
		g.genIntrinsicFn(name, d, inst.args)
		return
	}
	if d.Fn.Ast.Body == nil {
		g.fatalf("%s: error: no body for function `%s` (instance %s)", d.Span.Lo, d.Path(), typeck.Instance{Def: d, Args: inst.args})
	}
	r := g.tcx.CheckInstance(d, inst.args)
	if len(g.tcx.Errors) > 0 {
		g.fatalf("%s", g.tcx.Err())
	}
	fg := newFnGen(g, r)
	fg.computeSplit(d.Fn.Ast.Body)
	var params []string
	for i, pt := range r.ParamTys {
		params = append(params, fmt.Sprintf("%s p%d", g.ctype(pt), i))
	}
	sig := fmt.Sprintf("static %s %s(%s)", g.ctype(r.RetTy), name, joinParams(params))
	fmt.Fprintf(&g.protos, "%s;\n", sig)
	fg.retTy = r.RetTy
	fg.pushScope()
	for i, l := range r.Params {
		fg.declLocal(l)
		fg.line("%s = p%d;", fg.localName(l), i)
		fg.markInit(l)
	}
	// destructure pattern params
	fa := d.Fn.Ast
	pi := 0
	if fa.Decl.SelfParam != nil {
		pi = 1
	}
	for _, p := range fa.Decl.Params {
		l := r.Params[pi]
		pi++
		if l.Name == "__arg" {
			fg.bindOwnedPat(p.Pat, fg.localName(l), l.Ty, l, false)
		}
	}
	v := fg.blockValue(fa.Body, r.RetTy)
	fg.emitReturn(v)
	fg.popScopeNoDrop()
	fmt.Fprintf(&g.fns, "%s {\n%s}\n", sig, fg.b.String())
}

func joinParams(ps []string) string {
	if len(ps) == 0 {
		return "void"
	}
	return strings.Join(ps, ", ")
}

// genForeignDecl declares a foreign (extern "C") function under a private
// C name bound to the real symbol with an asm label, so that its Rust
// signature never conflicts with system headers.
func (g *Gen) genForeignDecl(name string, d *Def, args []Ty) {
	ins, out := g.tcx.InstantiateFnSig(d, args)
	var ps []string
	for _, in := range ins {
		g.defineByValue(in)
		ps = append(ps, g.ctype(in))
	}
	if g.tcx.FnSig(d).Variadic {
		ps = append(ps, "...")
	}
	ret := "void"
	if !isZST(out) {
		g.defineByValue(out)
		ret = g.ctype(out)
	}
	sym := d.Name
	for _, a := range d.Attrs {
		if a.Path == "link_name" && len(a.Tokens) >= 2 {
			sym = a.Tokens[1].Tok.Text
		}
	}
	fmt.Fprintf(&g.protos, "extern %s %s(%s) __asm__(%q);\n", ret, name, joinParams(ps), sym)
}

// constCached reports whether a const item's value is computed once and
// kept in static storage (it owns no resources, so every use may share it).
func (g *Gen) constCached(d *Def, t Ty) bool {
	return d.Kind == DefConst && !isZST(t) && !g.needsDrop(t)
}

// genCtorFn emits a constructor function for a tuple struct/variant.
func (g *Gen) genCtorFn(name string, d *Def, args []Ty) {
	ins, out := g.tcx.InstantiateFnSig(d, args)
	var ps []string
	var vals []string
	for i, in := range ins {
		ps = append(ps, fmt.Sprintf("%s a%d", g.ctype(in), i))
		vals = append(vals, fmt.Sprintf("a%d", i))
	}
	sig := fmt.Sprintf("static %s %s(%s)", g.ctype(out), name, joinParams(ps))
	fmt.Fprintf(&g.protos, "%s;\n", sig)
	fmt.Fprintf(&g.fns, "%s { return %s; }\n", sig, g.adtLiteral(out, d, vals))
}

// adtLiteral builds a compound literal for a struct/variant value.
func (g *Gen) adtLiteral(t Ty, d *Def, fields []string) string {
	g.defineByValue(t)
	ct := g.ctype(t)
	if t.Def.Adt.Kind == AdtEnum {
		v := d.Variant
		if n := g.niche(t); n != nil {
			if v == n.empty {
				return fmt.Sprintf("((%s){})", ct)
			}
			return fmt.Sprintf("((%s){ .f0 = %s })", ct, fields[0])
		}
		var fs []string
		for i, f := range fields {
			fs = append(fs, fmt.Sprintf(".f%d = %s", i, f))
		}
		if len(fs) == 0 {
			return fmt.Sprintf("((%s){ .tag = %d })", ct, v.Disc)
		}
		return fmt.Sprintf("((%s){ .tag = %d, .u.v%d = { %s } })", ct, v.Disc, v.Index, strings.Join(fs, ", "))
	}
	var fs []string
	for i, f := range fields {
		fs = append(fs, fmt.Sprintf(".f%d = %s", i, f))
	}
	return fmt.Sprintf("((%s){ %s })", ct, strings.Join(fs, ", "))
}

// genConstFn emits `T constN(void)` evaluating a const/static initializer.
func (g *Gen) genConstFn(name string, d *Def, args []Ty) {
	r := g.tcx.CheckInstance(d, args)
	if len(g.tcx.Errors) > 0 {
		g.fatalf("%s", g.tcx.Err())
	}
	fg := newFnGen(g, r)
	fg.inConst = true
	fg.retTy = r.RetTy
	sig := fmt.Sprintf("static %s %s(void)", g.ctype(r.RetTy), name)
	fmt.Fprintf(&g.protos, "%s;\n", sig)
	// a const's value is the same at every use: unless it owns resources
	// (each use must then get a fresh value), compute it once
	cache := g.constCached(d, r.RetTy)
	if cache {
		// file-scope cache, also reachable as a place via NAME_p()
		g.defineByValue(r.RetTy)
		fmt.Fprintf(&g.statics, "static %s %s_cache;\n", g.ctype(r.RetTy), name)
		fmt.Fprintf(&g.statics, "static uint8_t %s_done;\n", name)
		fmt.Fprintf(&g.protos, "static %s* %s_p(void);\n", g.ctype(r.RetTy), name)
		fmt.Fprintf(&g.fns, "static %s* %s_p(void) { (void)%s(); return &%s_cache; }\n", g.ctype(r.RetTy), name, name, name)
		fg.line("if (__atomic_load_n(&%s_done, __ATOMIC_ACQUIRE)) return %s_cache;", name, name)
	}
	fg.pushScope()
	v := fg.coerceValue(r.ConstExpr, r.RetTy)
	if cache {
		fg.line("%s_cache = %s;", name, v)
		fg.line("__atomic_store_n(&%s_done, 1, __ATOMIC_RELEASE);", name)
		v = name + "_cache"
	}
	fg.emitReturn(v)
	fg.popScopeNoDrop()
	fmt.Fprintf(&g.fns, "%s {\n%s}\n", sig, fg.b.String())
}

// staticVar returns the C global for a static item.
func (g *Gen) staticVar(d *Def) string {
	if n, ok := g.statics_[d]; ok {
		return n
	}
	g.fnN++
	name := fmt.Sprintf("g%d_%s", g.fnN, cIdent(d.Name))
	g.statics_[d] = name
	ty := g.tcx.Normalize(g.tcx.ConstTy(d))
	g.defineByValue(ty)
	fmt.Fprintf(&g.statics, "static %s %s;\n", g.ctype(ty), name)
	init := g.fnName(d, nil)
	g.staticInit = append(g.staticInit, fmt.Sprintf("%s = %s();", name, init))
	return name
}

// initArrayCtors handles `#[link_section = ".init_array"] static F: fn() = f;`
// (the `ctor` pattern): such functions run before main, which we emulate by
// calling them right after static initialization.
func (g *Gen) initArrayCtors() (calls []string) {
	for _, d := range g.tcx.R.Defs {
		if d.Kind != DefStatic || g.tcx.Generics(d).Count() != 0 {
			continue
		}
		section := ""
		for _, a := range d.Attrs {
			if a.Path == "link_section" && len(a.Tokens) >= 2 {
				section = strings.Trim(a.Tokens[1].Tok.Text, "\"")
			}
		}
		if !strings.Contains(section, "init_array") && !strings.Contains(section, ".ctors") {
			continue
		}
		ty := g.tcx.Normalize(g.tcx.ConstTy(d))
		name := g.staticVar(d)
		switch ty.Kind {
		case TFnPtr:
			calls = append(calls, fmt.Sprintf("%s();", name))
		case TArray:
			if ty.Elem.Kind == TFnPtr {
				calls = append(calls, fmt.Sprintf("for (size_t i = 0; i < sizeof(%s.a)/sizeof(%s.a[0]); i++) %s.a[i]();", name, name, name))
			}
		}
	}
	return calls
}

// closureFn returns the C function implementing a closure type's body.
func (g *Gen) closureFn(t Ty) string {
	if n, ok := g.closureFns[t]; ok {
		return n
	}
	g.fnN++
	name := fmt.Sprintf("closure%d", g.fnN)
	g.closureFns[t] = name
	g.closureQueue = append(g.closureQueue, t)
	ci := g.tcx.Closures[t]
	if ci == nil {
		g.fatalf("internal error: no closure info for callee type %s", t)
	}
	var ps []string
	ps = append(ps, g.ctype(t)+"* env")
	for i, in := range ci.Inputs {
		ps = append(ps, fmt.Sprintf("%s a%d", g.ctype(in), i))
	}
	fmt.Fprintf(&g.protos, "static %s %s(%s);\n", g.ctype(ci.Output), name, strings.Join(ps, ", "))
	return name
}

func (g *Gen) genClosure(t Ty) {
	ci := g.tcx.Closures[t]
	name := g.closureFns[t]
	g.defineByValue(t)
	fg := newFnGen(g, ci.Results)
	fg.computeSplit(ci.Expr.Body)
	fg.closure = ci
	fg.capIndex = map[*typeck.Local]int{}
	for i, c := range ci.Captures {
		fg.capIndex[c.Local] = i
	}
	fg.retTy = ci.Output
	var ps []string
	ps = append(ps, g.ctype(t)+"* env")
	for i, in := range ci.Inputs {
		ps = append(ps, fmt.Sprintf("%s a%d", g.ctype(in), i))
	}
	fg.pushScope()
	if ci.Kind == 2 {
		// an FnOnce closure consumes its environment: by-value captures not
		// moved out by the body are dropped when it returns
		for i, c := range ci.Captures {
			if !c.ByRef && g.needsDrop(c.Ty) {
				place := fmt.Sprintf("env->c%d", i)
				flag := fmt.Sprintf("dfc%d", i)
				fg.line("uint8_t %s = 1;", flag)
				fg.addDrop(place, flag, c.Ty)
				fg.line("RG_CS_PUSH(&%s, %s, &%s);", place, g.dropFn(c.Ty), flag)
			}
		}
	}
	for i, l := range ci.Params {
		fg.declLocal(l)
		fg.line("%s = a%d;", fg.localName(l), i)
		fg.markInit(l)
		if l.Name == "__arg" {
			fg.bindOwnedPat(ci.Expr.Params[i].Pat, fg.localName(l), l.Ty, l, false)
		}
	}
	v := fg.coerceValue(ci.Expr.Body, ci.Output)
	fg.emitReturn(v)
	fg.popScopeNoDrop()
	fmt.Fprintf(&g.fns, "static %s %s(%s) {\n%s}\n", g.ctype(ci.Output), name, strings.Join(ps, ", "), fg.b.String())
}

// fnValue returns a C expression naming the function behind a FnDef type
// (resolving trait methods).
func (g *Gen) fnValue(t Ty) string {
	d, args := g.resolveFn(t.Def, t.Args)
	return g.fnName(d, args)
}

// resolveFn maps trait methods to their implementations.
func (g *Gen) resolveFn(d *Def, args []Ty) (*Def, []Ty) {
	if d.InTrait() {
		rd, rargs, ok := g.tcx.ResolveAssocFn(d, args)
		if !ok || rd == nil {
			g.fatalf("%s: error: could not resolve trait method `%s` for `%s`", d.Span.Lo, d.Path(), args[0])
		}
		return rd, rargs
	}
	return d, args
}

// cString renders a Go string as a C string literal (bytes).
func cString(s string) string {
	var b strings.Builder
	b.WriteByte('"')
	for i := 0; i < len(s); i++ {
		c := s[i]
		switch {
		case c == '"':
			b.WriteString(`\"`)
		case c == '\\':
			b.WriteString(`\\`)
		case c == '\n':
			b.WriteString(`\n`)
		case c == '?':
			b.WriteString(`\?`)
		case c >= 32 && c < 127:
			b.WriteByte(c)
		default:
			fmt.Fprintf(&b, "\\%03o", c)
		}
	}
	b.WriteByte('"')
	return b.String()
}

func sortedKeys(m map[string]string) []string {
	var ks []string
	for k := range m {
		ks = append(ks, k)
	}
	sort.Strings(ks)
	return ks
}

var _ = ast.NewID
