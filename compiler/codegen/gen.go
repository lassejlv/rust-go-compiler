package codegen

import (
	_ "embed"
	"fmt"
	"os"
	"sort"
	"strings"

	"rustgo/compiler/ast"
	"rustgo/compiler/borrowck"
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
	locs    map[string]string // track_caller locations (locStatic)
	usesAsm bool              // asm! present (see Program.UsesAsm)
	// scalarConsts caches literal renderings of scalar const items.
	scalarConsts map[string]string
	fns          chunkBuf // function bodies (one chunk per function)

	types        map[Ty]*typeInfo
	typeN        int
	pendingTypes []Ty

	fnNames      map[string]string
	fnQueue      []instance
	fnN          int
	closureFns   map[Ty]string
	closureQueue []Ty

	dropFns        map[Ty]string
	boxFreeFns     map[Ty]string
	boxPartFns     map[boxPart]string
	dropQueue      []Ty
	needsDropCache map[Ty]bool
	dropImplCache  map[Ty]*instance

	vtables    map[string]string
	statics_   map[*Def]string
	staticInit []string
	strN       int

	Errors []string
	// bcSeen dedupes borrow checker diagnostics across instances.
	bcSeen    map[string]bool
	bcChecked map[*Def]bool
}

// New creates a generator.
func New(tcx *typeck.Tcx) *Gen {
	return &Gen{
		tcx:            tcx,
		types:          map[Ty]*typeInfo{},
		fnNames:        map[string]string{},
		closureFns:     map[Ty]string{},
		dropFns:        map[Ty]string{},
		boxFreeFns:     map[Ty]string{},
		boxPartFns:     map[boxPart]string{},
		needsDropCache: map[Ty]bool{},
		dropImplCache:  map[Ty]*instance{},
		vtables:        map[string]string{},
		statics_:       map[*Def]string{},
		scalarConsts:   map[string]string{},
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
	if d := g.tcx.R.StdPath("rt::runtime_panic"); d != nil && d.Kind == DefFn {
		rp := g.fnName(d, nil)
		g.staticInit = append([]string{fmt.Sprintf("rg_std_panic = (void (*)(Fat, Fat, uint32_t, uint32_t, uint8_t))%s;", rp)}, g.staticInit...)
	}
	ctors := g.initArrayCtors()
	g.drain()
	g.checkUnreached()
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
		types:    g.fwd.String() + g.defs.String(),
		protos:   g.protos.String(),
		data:     g.statics.String() + g.vtdefs.String(),
		fns:      g.fns.String(),
		fnEnds:   g.fns.ends,
		main:     m.String(),
		UsesAsm:  g.usesAsm,
		LinkArgs: append([]string(nil), g.tcx.R.LinkArgs...),
	}, nil
}

type cgError struct{ msg string }

// checkUnreached type- and borrow-checks the non-generic functions of local
// crates that code generation did not reach (rustc checks every function).
func (g *Gen) checkUnreached() {
	g.checkUnreachedFns()
	if len(g.tcx.Errors) == 0 && len(g.Errors) == 0 {
		// generic bodies, polymorphically (after every instance, whose
		// errors are more precise)
		g.tcx.CheckGenericBodies()
	}
}

func (g *Gen) checkUnreachedFns() {
	if os.Getenv("RUSTGO_BORROWCK") == "0" {
		return
	}
	for _, d := range g.tcx.R.Defs {
		if d.Kind != DefFn || d.Fn == nil || d.Fn.Ast == nil || d.Fn.Ast.Body == nil || d.Crate == nil || !d.Crate.Local {
			continue
		}
		if g.bcChecked[d] || g.tcx.Generics(d).Count() != 0 {
			continue
		}
		r := g.tcx.CheckInstance(d, nil)
		if len(g.tcx.Errors) > 0 {
			return
		}
		g.borrowck(d, r)
	}
}

// borrowck runs the borrow checker on a function instance of a local crate
// (RUSTGO_BORROWCK=all: every crate; =0: off).
func (g *Gen) borrowck(d *Def, r *typeck.Results) {
	mode := os.Getenv("RUSTGO_BORROWCK")
	if mode == "0" {
		return
	}
	if mode != "all" && (d.Crate == nil || !d.Crate.Local) {
		return
	}
	if g.bcSeen == nil {
		g.bcSeen = map[string]bool{}
		g.bcChecked = map[*Def]bool{}
	}
	g.bcChecked[d] = true
	for _, m := range borrowck.CheckFn(g.tcx, d, r) {
		if !g.bcSeen[m] {
			g.bcSeen[m] = true
			g.Errors = append(g.Errors, m)
		}
	}
}

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
	g.borrowck(d, r)
	fg := newFnGen(g, r)
	fg.computeSplit(d.Fn.Ast.Body)
	if hasTrackCaller(d) {
		// the caller stored its location right before calling us
		fg.trackCaller = true
		fg.line("const RgLoc* __loc = rg_caller_loc; int __loc_tracked = rg_caller_tracked;")
	}
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
		return g.enumLiteral(t, d.Variant, fields)
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
	var v string
	if e := r.ConstExpr; constAggregate(e) && fg.promotable(e) && len(fg.r.Adjust[e.GetID()]) == 0 && fg.ty(e) == r.RetTy {
		// a constant table (`static T: [(u32, u32); 900] = [...]`):
		// static data instead of a compound literal built at run time,
		// which costs the C compiler a lot for large tables
		inner := fg.constC(e)
		k := fg.newName("cinit")
		fmt.Fprintf(&g.statics, "static %s %s = %s;\n", g.ctype(r.RetTy), k, inner)
		v = k
	} else {
		v = fg.coerceValue(r.ConstExpr, r.RetTy)
	}
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
			if a.Path == "link_section" {
				section = attrStringValue(a)
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

// hasTrackCaller reports #[track_caller] on a fn (or on the trait method an
// impl method implements).
func hasTrackCaller(d *Def) bool {
	if d == nil {
		return false
	}
	for _, a := range d.Attrs {
		if a.Path == "track_caller" {
			return true
		}
	}
	if d.Parent != nil && d.Parent.Kind == DefImpl && d.Parent.Impl != nil && d.Parent.Impl.TraitRef != nil {
		if td, ok := d.Parent.Impl.TraitRef.Trait.Trait.Items[d.Name]; ok && td != d {
			for _, a := range td.Attrs {
				if a.Path == "track_caller" {
					return true
				}
			}
		}
	}
	return false
}

// locStatic returns a static core::panic::Location for a source position.
func (g *Gen) locStatic(sp lexer.Span) string {
	file := lexer.FileName(sp.Lo.File)
	key := fmt.Sprintf("%s:%d:%d", file, sp.Lo.Line, sp.Lo.Col)
	if n, ok := g.locs[key]; ok {
		return n
	}
	g.fnN++
	name := fmt.Sprintf("rgloc%d", g.fnN)
	if g.locs == nil {
		g.locs = map[string]string{}
	}
	g.locs[key] = name
	fmt.Fprintf(&g.statics, "static RgLoc %s = { { (void*)%s, %d }, %d, %d };\n", name, cString(file), len(file), sp.Lo.Line, sp.Lo.Col)
	return name
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
	fg.pathCaps = map[*typeck.Local]bool{}
	fg.ownedUpvars = map[*typeck.Local]bool{}
	for i, c := range ci.Captures {
		if len(c.Path) == 0 {
			fg.capIndex[c.Local] = i
		} else {
			fg.pathCaps[c.Local] = true
		}
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
		// moved out by the body are dropped when it returns (or unwinds),
		// in the order of the environment's fields like rustc: registered
		// last-first, as drops run in reverse
		for i := len(ci.Captures) - 1; i >= 0; i-- {
			c := ci.Captures[i]
			if !c.ByRef && g.needsDrop(c.Ty) && len(c.Path) == 0 && len(fg.split[c.Local]) > 0 && fg.splittable(c.Ty) {
				// fields are moved out of it: per-field drop flags, as
				// for a local (named after the captured variable)
				l := c.Local
				fg.ownedUpvars[l] = true
				place := fmt.Sprintf("env->c%d", i)
				n := fg.declSplitFlags(l, place, 1)
				sc := fg.scopes[len(fg.scopes)-1]
				sc.drops = append(sc.drops, dropEntry{place: place, flag: fg.flagName(l), ty: l.Ty, local: l, cs: n})
				continue
			}
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

// attrStringValue returns the string of `#[name = "value"]` (looking through
// macro-fragment groups around the value).
func attrStringValue(a *ast.Attr) string {
	var find func(ts []ast.TokenTree) string
	find = func(ts []ast.TokenTree) string {
		for _, t := range ts {
			if t.Delim == 0 && t.Tok.Kind == lexer.Str {
				return strings.Trim(t.Tok.Text, "\"")
			}
			if t.Delim != 0 {
				if v := find(t.Inner); v != "" {
					return v
				}
			}
		}
		return ""
	}
	return find(a.Tokens)
}

// constAggregate reports whether e is an array or tuple expression (the
// initializers worth emitting as static data).
func constAggregate(e ast.Expr) bool {
	switch unparen(e).(type) {
	case *ast.ArrayExpr, *ast.TupleExpr:
		return true
	}
	return false
}
