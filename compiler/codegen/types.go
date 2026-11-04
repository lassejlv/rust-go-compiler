// Package codegen lowers type-checked, monomorphized Rust to GNU C
// (playing the role of rustc_codegen_llvm, with C as the target IR).
package codegen

import (
	"fmt"
	"strings"

	"rustgo/compiler/ast"
	. "rustgo/compiler/middle"
)

// ctype returns the C type name for a Rust type (defining it if needed).
func (g *Gen) ctype(t Ty) string {
	switch t.Kind {
	case TBool:
		return "uint8_t"
	case TChar:
		return "uint32_t"
	case TInt:
		switch t.IntKind() {
		case I8:
			return "int8_t"
		case I16:
			return "int16_t"
		case I32:
			return "int32_t"
		case I64, Isize:
			return "int64_t"
		case I128:
			return "__int128"
		case U8:
			return "uint8_t"
		case U16:
			return "uint16_t"
		case U32:
			return "uint32_t"
		case U64, Usize:
			return "uint64_t"
		case U128:
			return "unsigned __int128"
		}
	case TFloat:
		if t.Sub == 32 {
			return "float"
		}
		return "double"
	case TNever:
		return "Unit"
	case TFnDef:
		return "Unit"
	case TTuple:
		if len(t.Elems) == 0 {
			return "Unit"
		}
	case TRef, TPtr:
		e := t.Elem
		if e.IsUnsized() {
			return "Fat"
		}
		return g.ctype(e) + "*"
	case TStr, TSlice, TDyn:
		// unsized values only appear behind pointers; give a byte type so
		// that `T*` style uses still compile.
		return "uint8_t"
	case TError, TParam, TInfer, TProjection, TOpaque:
		panic(fmt.Sprintf("codegen: unexpected type %s", t))
	}
	return g.namedType(t)
}

// cty is a shorthand that also normalizes.
func (g *Gen) cty(t Ty) string { return g.ctype(t) }

type typeInfo struct {
	name     string
	defined  bool
	defining bool
}

func (g *Gen) namedType(t Ty) string {
	if ti, ok := g.types[t]; ok {
		return ti.name
	}
	g.typeN++
	name := fmt.Sprintf("T%d", g.typeN)
	ti := &typeInfo{name: name}
	g.types[t] = ti
	kw := "struct"
	if t.Kind == TAdt && t.Def.Adt.Kind == AdtUnion {
		kw = "union"
	}
	if t.Kind == TFnPtr {
		// function pointer typedef
		var ps []string
		for _, in := range t.Elems {
			ps = append(ps, g.ctype(in))
		}
		if len(ps) == 0 {
			ps = append(ps, "void")
		}
		if t.FnVariadic() {
			ps = append(ps, "...")
		}
		fmt.Fprintf(&g.fwd, "typedef %s (*%s)(%s); /* %s */\n", g.ctype(t.Ret), name, strings.Join(ps, ", "), sanitizeComment(t.String()))
		ti.defined = true
		return name
	}
	fmt.Fprintf(&g.fwd, "typedef %s %s %s; /* %s */\n", kw, name, name, sanitizeComment(t.String()))
	g.checkTypeDepth(t, false)
	g.pendingTypes = append(g.pendingTypes, t)
	return name
}

func sanitizeComment(s string) string {
	return strings.ReplaceAll(strings.ReplaceAll(s, "*/", "* /"), "/*", "/ *")
}

// defineType emits the full definition of a named type (and, first, of all
// types it contains by value).
func (g *Gen) defineType(t Ty) {
	if t.Kind == TArray {
		g.defineType(t.Elem)
	}
	ti, ok := g.types[t]
	if !ok {
		g.ctype(t)
		ti, ok = g.types[t]
		if !ok {
			return
		}
	}
	if ti.defined || ti.defining {
		return
	}
	ti.defining = true
	var b strings.Builder
	switch t.Kind {
	case TTuple:
		for _, e := range t.Elems {
			g.defineByValue(e)
		}
		fmt.Fprintf(&b, "struct %s {", ti.name)
		for _, i := range g.tcx.FieldOrder(t, 0) {
			fmt.Fprintf(&b, " %s f%d;", g.ctype(t.Elems[i]), i)
		}
		b.WriteString(" };\n")
	case TArray:
		g.defineByValue(t.Elem)
		fmt.Fprintf(&b, "struct %s { %s a[%d]; };\n", ti.name, g.ctype(t.Elem), g.arrayLen(t))
	case TClosure:
		ci := g.tcx.Closures[t]
		for _, c := range ci.Captures {
			if !c.ByRef {
				g.defineByValue(c.Ty)
			}
		}
		fmt.Fprintf(&b, "struct %s {", ti.name)
		for _, i := range g.tcx.FieldOrder(t, 0) {
			c := ci.Captures[i]
			if c.ByRef {
				fmt.Fprintf(&b, " %s* c%d;", g.ctype(c.Ty), i)
			} else {
				fmt.Fprintf(&b, " %s c%d;", g.ctype(c.Ty), i)
			}
		}
		b.WriteString(" };\n")
	case TAdt:
		d := t.Def
		g.tcx.AdtFields(d)
		switch d.Adt.Kind {
		case AdtStruct, AdtUnion:
			kw := "struct"
			if d.Adt.Kind == AdtUnion {
				kw = "union"
			}
			fts := g.fieldTypes(t, 0)
			for _, ft := range fts {
				if !ft.IsUnsized() {
					g.defineByValue(ft)
				}
			}
			fmt.Fprintf(&b, "%s %s {", kw, ti.name)
			order := g.tcx.FieldOrder(t, 0)
			for pos, i := range order {
				ft := fts[i]
				if ft.IsUnsized() {
					if pos == 0 {
						b.WriteString(" uint8_t __rg_hdr[0];")
					}
					// unsized tail: flexible array member
					switch ft.Kind {
					case TStr:
						fmt.Fprintf(&b, " uint8_t f%d[];", i)
					case TSlice:
						g.defineByValue(ft.Elem)
						fmt.Fprintf(&b, " %s f%d[];", g.ctype(ft.Elem), i)
					default:
						// dyn / nested unsized struct: placed at a dynamic offset
						fmt.Fprintf(&b, " uint8_t f%d[];", i)
					}
					continue
				}
				if pk := d.ReprPacked(); pk > 1 {
					// packed(N): fields aligned to min(align, N)
					_, fa, _ := g.tcx.CLayout(ft)
					fmt.Fprintf(&b, " %s f%d __attribute__((aligned(%d)));", g.ctype(ft), i, min(fa, pk))
					continue
				}
				fmt.Fprintf(&b, " %s f%d;", g.ctype(ft), i)
			}
			switch {
			case reprPacked(d):
				b.WriteString(" } __attribute__((packed));\n")
			case d.ReprAlign() > 0:
				fmt.Fprintf(&b, " } __attribute__((aligned(%d)));\n", d.ReprAlign())
			default:
				b.WriteString(" };\n")
			}
		case AdtEnum:
			for vi := range d.Adt.Variants {
				for _, ft := range g.fieldTypes(t, vi) {
					g.defineByValue(ft)
				}
			}
			g.enumDef(&b, t, ti.name)
		}
	default:
		ti.defining = false
		ti.defined = true
		return
	}
	g.defs.WriteString(b.String())
	ti.defining = false
	ti.defined = true
}

// defineByValue ensures a type used by value is fully defined.
func (g *Gen) defineByValue(t Ty) {
	switch t.Kind {
	case TTuple:
		if len(t.Elems) == 0 {
			return
		}
		g.defineType(t)
	case TArray, TAdt, TClosure:
		g.defineType(t)
	}
}

func (g *Gen) arrayLen(t Ty) uint64 {
	if t.Len.Kind != TConst {
		panic(fmt.Sprintf("codegen: array length not known: %s", t))
	}
	return t.Len.Val
}

// fieldTypes returns the concrete field types of variant vi of an ADT type.
func (g *Gen) fieldTypes(t Ty, vi int) []Ty {
	d := t.Def
	g.tcx.AdtFields(d)
	v := d.Adt.Variants[vi]
	out := make([]Ty, len(v.Fields))
	for i, f := range v.Fields {
		out[i] = g.tcx.Normalize(Subst(f.Ty, t.Args))
	}
	return out
}

func (g *Gen) variantIndex(v *VariantDef) int { return v.Index }

// isFreeze reports whether t is Freeze: it contains no UnsafeCell except
// behind a reference or pointer (the auto trait's structural rule).
func (g *Gen) isFreeze(t Ty, depth int) bool {
	if depth > 64 {
		return true
	}
	switch t.Kind {
	case TTuple:
		for _, e := range t.Elems {
			if !g.isFreeze(e, depth+1) {
				return false
			}
		}
		return true
	case TArray, TSlice:
		return g.isFreeze(t.Elem, depth+1)
	case TClosure:
		ci := g.tcx.Closures[t]
		for _, c := range ci.Captures {
			if !c.ByRef && !g.isFreeze(c.Ty, depth+1) {
				return false
			}
		}
		return true
	case TAdt:
		d := t.Def
		if d.LangItem == "unsafe_cell" {
			return false
		}
		if d.LangItem == "phantom_data" {
			return true
		}
		for vi := range d.Adt.Variants {
			for _, ft := range g.fieldTypes(t, vi) {
				if !g.isFreeze(ft, depth+1) {
					return false
				}
			}
		}
		return true
	}
	return true
}

// ---------------------------------------------------------------------------
// Drop glue

// needsDrop reports whether values of t require drop glue.
func (g *Gen) needsDrop(t Ty) bool {
	if v, ok := g.needsDropCache[t]; ok {
		return v
	}
	g.needsDropCache[t] = false // break cycles
	r := g.computeNeedsDrop(t)
	g.needsDropCache[t] = r
	return r
}

func (g *Gen) computeNeedsDrop(t Ty) bool {
	switch t.Kind {
	case TTuple:
		for _, e := range t.Elems {
			if g.needsDrop(e) {
				return true
			}
		}
		return false
	case TArray:
		if g.arrayLen(t) == 0 {
			return false
		}
		return g.needsDrop(t.Elem)
	case TSlice:
		return g.needsDrop(t.Elem)
	case TDyn:
		return true
	case TClosure:
		ci := g.tcx.Closures[t]
		for _, c := range ci.Captures {
			if !c.ByRef && g.needsDrop(c.Ty) {
				return true
			}
		}
		return false
	case TAdt:
		d := t.Def
		if d.LangItem == "manually_drop" {
			return false
		}
		if g.dropImpl(t) != nil {
			return true
		}
		if d.Adt.Kind == AdtUnion {
			return false
		}
		for vi := range d.Adt.Variants {
			for _, ft := range g.fieldTypes(t, vi) {
				if g.needsDrop(ft) {
					return true
				}
			}
		}
		return false
	}
	return false
}

// dropImpl returns the Drop::drop impl method instance for t, if any.
func (g *Gen) dropImpl(t Ty) *instance {
	if v, ok := g.dropImplCache[t]; ok {
		return v
	}
	var res *instance
	if dt := g.tcx.LangOpt("drop"); dt != nil && t.Kind == TAdt {
		m := dt.Trait.Items["drop"]
		if d, args, ok := g.tcx.ResolveAssocFn(m, []Ty{t}); ok && d != nil && d != m {
			res = &instance{def: d, args: args}
		}
	}
	g.dropImplCache[t] = res
	return res
}

// dropFn returns the name of the drop glue function `void drop_X(T* p)`.
func (g *Gen) dropFn(t Ty) string {
	if name, ok := g.dropFns[t]; ok {
		return name
	}
	g.fnN++
	name := fmt.Sprintf("drop%d", g.fnN)
	g.dropFns[t] = name
	g.checkTypeDepth(t, true)
	g.dropQueue = append(g.dropQueue, t)
	return name
}

// boxFree renders the deallocation of the heap block of box (a C place of
// type Box<T>, T sized) without dropping its contents.
func (g *Gen) boxFree(box string, t Ty) string {
	g.defineByValue(t.Args[0])
	ct := g.ctype(t.Args[0])
	return fmt.Sprintf("if (sizeof(%s)) rg_dealloc((uint8_t*)%s.f0, sizeof(%s), _Alignof(%s));", ct, box, ct, ct)
}

// boxFreeFn returns the name of `void f(Box<T>* p)` freeing the heap block
// of a box whose contents were moved out or dropped separately (cleanup
// stack entry of a partially moved box).
func (g *Gen) boxFreeFn(t Ty) string {
	if name, ok := g.boxFreeFns[t]; ok {
		return name
	}
	g.fnN++
	name := fmt.Sprintf("boxfree%d", g.fnN)
	g.boxFreeFns[t] = name
	g.defineByValue(t)
	g.defineByValue(t.Args[0])
	pt := g.ctype(t) + "*"
	fmt.Fprintf(&g.protos, "static void %s(%s p);\n", name, pt)
	fmt.Fprintf(&g.fns, "static void %s(%s p) { %s }\n", name, pt, g.boxFree("(*p)", t))
	return name
}

type boxPart struct {
	box Ty
	sel string
}

// boxPartFn returns the name of `void f(Box<T>* p)` dropping the part sel
// (a C selector like ".f1.f0", of type ft) of the box's contents (cleanup
// stack entry of a partially moved box: the box pointer is only read when
// the entry runs).
func (g *Gen) boxPartFn(t Ty, sel string, ft Ty) string {
	k := boxPart{t, sel}
	if name, ok := g.boxPartFns[k]; ok {
		return name
	}
	g.fnN++
	name := fmt.Sprintf("boxpart%d", g.fnN)
	g.boxPartFns[k] = name
	g.defineByValue(t)
	g.defineByValue(t.Args[0])
	pt := g.ctype(t) + "*"
	fmt.Fprintf(&g.protos, "static void %s(%s p);\n", name, pt)
	fmt.Fprintf(&g.fns, "static void %s(%s p) { %s(&(*p->f0)%s); }\n", name, pt, g.dropFn(ft), sel)
	return name
}

func (g *Gen) emitDropGlue(t Ty) {
	name := g.dropFns[t]
	var b strings.Builder
	pt := g.ctype(t) + "*"
	if t.IsUnsized() {
		pt = "Fat"
	}
	fmt.Fprintf(&g.protos, "static void %s(%s p);\n", name, pt)
	fmt.Fprintf(&b, "static void %s(%s p) { /* %s */\n", name, pt, sanitizeComment(t.String()))
	switch t.Kind {
	case TTuple:
		for i, e := range t.Elems {
			if g.needsDrop(e) {
				fmt.Fprintf(&b, "  %s(&p->f%d);\n", g.dropFn(e), i)
			}
		}
	case TArray:
		fmt.Fprintf(&b, "  for (uint64_t i = 0; i < %d; i++) %s(&p->a[i]);\n", g.arrayLen(t), g.dropFn(t.Elem))
	case TSlice:
		fmt.Fprintf(&b, "  for (uint64_t i = 0; i < p.meta; i++) %s(&((%s*)p.ptr)[i]);\n", g.dropFn(t.Elem), g.ctype(t.Elem))
	case TDyn:
		b.WriteString("  ((void(*)(void*))((void**)p.meta)[0])(p.ptr);\n")
	case TClosure:
		ci := g.tcx.Closures[t]
		for i, c := range ci.Captures {
			if !c.ByRef && g.needsDrop(c.Ty) {
				fmt.Fprintf(&b, "  %s(&p->c%d);\n", g.dropFn(c.Ty), i)
			}
		}
	case TAdt:
		if t.IsUnsized() {
			g.defineType(t)
			sn := g.ctype(t)
			fts := g.fieldTypes(t, 0)
			for i, ft := range fts {
				if !g.needsDrop(ft) {
					continue
				}
				if ft.IsUnsized() {
					if staticTail(ft) {
						fmt.Fprintf(&b, "  %s((Fat){ (void*)((%s*)p.ptr)->f%d, p.meta });\n", g.dropFn(ft), sn, i)
					} else {
						fmt.Fprintf(&b, "  %s((Fat){ (void*)((uint8_t*)p.ptr + %s), p.meta });\n", g.dropFn(ft), g.tailOffset(t, "p.meta"))
					}
				} else {
					fmt.Fprintf(&b, "  %s(&((%s*)p.ptr)->f%d);\n", g.dropFn(ft), sn, i)
				}
			}
			break
		}
		if di := g.dropImpl(t); di != nil {
			fmt.Fprintf(&b, "  %s(p);\n", g.fnName(di.def, di.args))
		}
		d := t.Def
		switch d.Adt.Kind {
		case AdtStruct:
			for i, ft := range g.fieldTypes(t, 0) {
				if g.needsDrop(ft) {
					fmt.Fprintf(&b, "  %s(&p->f%d);\n", g.dropFn(ft), i)
				}
			}
		case AdtEnum:
			fmt.Fprintf(&b, "  switch (%s) {\n", g.enumTag(t, "(*p)"))
			for vi, v := range d.Adt.Variants {
				var drops []string
				for i, ft := range g.fieldTypes(t, vi) {
					if g.needsDrop(ft) {
						drops = append(drops, fmt.Sprintf("%s(&%s.f%d);", g.dropFn(ft), g.enumPayload(t, "(*p)", vi), i))
					}
				}
				if len(drops) > 0 {
					fmt.Fprintf(&b, "  case %d: %s break;\n", v.Disc, strings.Join(drops, " "))
				}
			}
			b.WriteString("  default: break;\n  }\n")
		}
	}
	b.WriteString("}\n")
	g.fns.WriteString(b.String())
}

// ---------------------------------------------------------------------------
// Vtables

// vtable returns the name of the vtable for (concrete type, dyn type).
func (g *Gen) vtable(concrete, dyn Ty) string {
	key := concrete.Key() + "|" + dyn.Key()
	if name, ok := g.vtables[key]; ok {
		return name
	}
	g.fnN++
	name := fmt.Sprintf("vt%d", g.fnN)
	g.vtables[key] = name
	var entries []string
	if g.needsDrop(concrete) {
		entries = append(entries, "(void*)"+g.dropFn(concrete))
	} else {
		entries = append(entries, "(void*)rg_drop_noop")
	}
	g.defineByValue(concrete)
	entries = append(entries, fmt.Sprintf("(void*)(uintptr_t)sizeof(%s)", g.ctype(concrete)), fmt.Sprintf("(void*)(uintptr_t)_Alignof(%s)", g.ctype(concrete)))
	for _, m := range g.vtableMethods(dyn) {
		entries = append(entries, g.vtableEntry(concrete, dyn, m))
	}
	for _, st := range g.supertraitSlots(dyn) {
		entries = append(entries, "(void*)"+g.vtable(concrete, g.supertraitDyn(dyn, st)))
	}
	fmt.Fprintf(&g.protos, "static void* %s[%d];\n", name, len(entries))
	fmt.Fprintf(&g.vtdefs, "static void* %s[%d] = {%s};\n", name, len(entries), strings.Join(entries, ", "))
	return name
}

// vtableMethods lists the methods (trait method defs with their trait ref
// args, Self = dyn) in vtable order.
func (g *Gen) vtableMethods(dyn Ty) []vtMethod {
	if dyn.Def == nil {
		return nil
	}
	obj := &TraitRef{Trait: dyn.Def, Args: append([]Ty{dyn}, dyn.Args...)}
	var out []vtMethod
	for _, st := range g.tcx.AllSupertraits(obj) {
		for _, it := range st.Trait.Trait.Order {
			if it.Kind == DefFn && it.Fn.HasSelf {
				out = append(out, vtMethod{def: it, traitArgs: st.Args})
			}
		}
	}
	return out
}

type vtMethod struct {
	def       *Def
	traitArgs []Ty
}

// vtableIndex returns the vtable slot of a trait method for a dyn type.
func (g *Gen) vtableIndex(dyn Ty, m *Def) int {
	for i, vm := range g.vtableMethods(dyn) {
		if vm.def == m {
			return 3 + i
		}
	}
	panic("method not in vtable: " + m.Name)
}

func (g *Gen) vtableEntry(concrete, dyn Ty, m vtMethod) string {
	// Fn-family traits on closures / fn items / fn pointers map to the
	// callable itself.
	ln := m.def.Parent.LangItem
	if ln == "fn" || ln == "fn_mut" || ln == "fn_once" {
		if ln == "fn_once" && dyn.Def != nil && dyn.Def.LangItem == "fn_once" {
			return "(void*)" + g.callOnceThunk(concrete)
		}
		return "(void*)" + g.callableThunk(concrete)
	}
	// skip generic methods and methods requiring Self: Sized
	mg := g.tcx.Generics(m.def)
	if len(mg.Own()) > 0 || requiresSized(m.def) {
		return "0"
	}
	args := append([]Ty{concrete}, m.traitArgs[1:]...)
	d, iargs, ok := g.tcx.ResolveAssocFn(m.def, args)
	if !ok || d == nil {
		return "0"
	}
	return "(void*)" + g.fnName(d, iargs)
}

func requiresSized(d *Def) bool {
	fa := d.Fn.Ast
	if fa.Generics == nil {
		return false
	}
	for _, w := range fa.Generics.Where {
		if pt, ok := w.Ty.(*ast.PathType); ok && len(pt.Path.Segments) == 1 && pt.Path.Segments[0].Ident.Name == "Self" {
			for _, b := range w.Bounds {
				if b.Path != nil && len(b.Path.Segments) > 0 && b.Path.Segments[len(b.Path.Segments)-1].Ident.Name == "Sized" {
					return true
				}
			}
		}
	}
	return false
}

// callableThunk returns a function `Ret f(void* self, args...)` invoking a
// callable value (closure, fn item, fn pointer) stored at self.
func (g *Gen) callableThunk(t Ty) string {
	key := "thunk|" + t.Key()
	if name, ok := g.vtables[key]; ok {
		return name
	}
	switch t.Kind {
	case TClosure:
		name := g.closureFn(t)
		g.vtables[key] = name
		return name
	}
	g.fnN++
	name := fmt.Sprintf("thunk%d", g.fnN)
	g.vtables[key] = name
	var ins []Ty
	var out Ty
	switch t.Kind {
	case TFnDef:
		ins, out = g.tcx.InstantiateFnSig(t.Def, t.Args)
	case TFnPtr:
		ins, out = t.Elems, t.Ret
	default:
		panic("callableThunk: unsupported " + t.String())
	}
	var ps, as []string
	ps = append(ps, "void* self")
	for i, in := range ins {
		ps = append(ps, fmt.Sprintf("%s a%d", g.ctype(in), i))
		as = append(as, fmt.Sprintf("a%d", i))
	}
	var callee string
	if t.Kind == TFnDef {
		callee = g.fnValue(t)
	} else {
		callee = fmt.Sprintf("(*(%s*)self)", g.ctype(t))
	}
	fmt.Fprintf(&g.protos, "static %s %s(%s);\n", g.ctype(out), name, strings.Join(ps, ", "))
	fmt.Fprintf(&g.fns, "static %s %s(%s) { return %s(%s); }\n", g.ctype(out), name, strings.Join(ps, ", "), callee, strings.Join(as, ", "))
	return name
}

// callOnceThunk is the `dyn FnOnce` call slot: the call consumes the
// callable, so after an Fn/FnMut-kind closure returns (or unwinds) its
// environment is dropped. (FnOnce-kind closure bodies drop their own
// environment.)
func (g *Gen) callOnceThunk(t Ty) string {
	if t.Kind != TClosure || !g.needsDrop(t) {
		return g.callableThunk(t)
	}
	ci := g.tcx.Closures[t]
	if ci == nil || ci.Kind == 2 {
		return g.callableThunk(t)
	}
	key := "oncethunk|" + t.Key()
	if name, ok := g.vtables[key]; ok {
		return name
	}
	g.fnN++
	name := fmt.Sprintf("thunk%d", g.fnN)
	g.vtables[key] = name
	cf := g.closureFn(t)
	ps := []string{"void* self"}
	as := []string{fmt.Sprintf("(%s*)self", g.ctype(t))}
	for i, in := range ci.Inputs {
		ps = append(ps, fmt.Sprintf("%s a%d", g.ctype(in), i))
		as = append(as, fmt.Sprintf("a%d", i))
	}
	drop := g.dropFn(t)
	rt := g.ctype(ci.Output)
	fmt.Fprintf(&g.protos, "static %s %s(%s);\n", rt, name, strings.Join(ps, ", "))
	fmt.Fprintf(&g.fns, "static %s %s(%s) { uint8_t fl = 1; RG_CS_PUSH(self, %s, &fl); %s r = %s(%s); fl = 0; RG_CS_POP(1); %s(self); return r; }\n",
		rt, name, strings.Join(ps, ", "), drop, rt, cf, strings.Join(as, ", "), drop)
	return name
}

// reprPacked reports #[repr(packed)] / #[repr(C, packed)].
func reprPacked(d *Def) bool {
	for _, a := range d.Attrs {
		if a.Path == "repr" && len(a.Tokens) > 0 && a.Tokens[0].Delim == '(' {
			for _, tt := range a.Tokens[0].Inner {
				if tt.Tok.Text == "packed" {
					return true
				}
			}
		}
	}
	return false
}
