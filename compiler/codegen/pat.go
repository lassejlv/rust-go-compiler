package codegen

import (
	"fmt"
	"strings"

	"rustgo/compiler/ast"
	"rustgo/compiler/lexer"
	. "rustgo/compiler/middle"
	"rustgo/compiler/typeck"
)

func fileNameOf(sp lexer.Span) string { return lexer.FileName(sp.Lo.File) }

// pplace is a place inside a pattern match. For unsized types the string is
// the fat pointer value.
type pplace struct {
	s       string
	unsized bool
}

// patDerefs applies the implicit dereferences recorded for p.
func (fc *fnGen) patDerefs(p ast.Pat, pl pplace, t Ty) (pplace, Ty) {
	for _, rt := range fc.r.PatDerefs[p.GetID()] {
		if rt.Elem.IsUnsized() {
			pl = pplace{s: pl.s, unsized: true}
		} else {
			pl = pplace{s: "(*" + pl.s + ")"}
		}
		t = rt.Elem
	}
	return pl, t
}

func (fc *fnGen) patTest(p ast.Pat, place string, t Ty) string {
	return fc.ptest(p, pplace{s: place, unsized: t.IsUnsized()}, t)
}

func andAll(parts []string) string {
	var keep []string
	for _, p := range parts {
		if p != "1" {
			keep = append(keep, p)
		}
	}
	if len(keep) == 0 {
		return "1"
	}
	return "(" + strings.Join(keep, " && ") + ")"
}

func (fc *fnGen) ptest(p ast.Pat, pl pplace, t Ty) string {
	pl, t = fc.patDerefs(p, pl, t)
	switch x := p.(type) {
	case *ast.WildPat, *ast.RestPat:
		return "1"
	case *ast.ParenPat:
		return fc.ptest(x.Pat, pl, t)
	case *ast.IdentPat:
		if vr := fc.r.Paths[x.ID]; vr != nil {
			return fc.pathTest(vr, pl, t)
		}
		if x.Sub != nil {
			return fc.ptest(x.Sub, pl, t)
		}
		return "1"
	case *ast.LitPat:
		return fc.litTest(x.Expr, pl, t)
	case *ast.RangePat:
		var parts []string
		if x.Lo != nil {
			parts = append(parts, fmt.Sprintf("%s >= %s", pl.s, fc.value(x.Lo)))
		}
		if x.Hi != nil {
			op := "<"
			if x.Inclusive {
				op = "<="
			}
			parts = append(parts, fmt.Sprintf("%s %s %s", pl.s, op, fc.value(x.Hi)))
		}
		return andAll(parts)
	case *ast.TuplePat:
		var parts []string
		fc.seqFields(x.Elems, len(t.Elems), func(pi, fi int) {
			parts = append(parts, fc.ptest(x.Elems[pi], pplace{s: fmt.Sprintf("%s.f%d", pl.s, fi)}, t.Elems[fi]))
		})
		return andAll(parts)
	case *ast.TupleStructPat:
		v := fc.r.Variants[x.ID]
		fts := fc.g.fieldTypes(t, v.Index)
		var parts []string
		prefix := pl.s
		if t.Def.Adt.Kind == AdtEnum {
			parts = append(parts, fmt.Sprintf("%s.tag == %d", pl.s, v.Disc))
			prefix = fmt.Sprintf("%s.u.v%d", pl.s, v.Index)
		}
		fc.seqFields(x.Elems, len(fts), func(pi, fi int) {
			parts = append(parts, fc.ptest(x.Elems[pi], pplace{s: fmt.Sprintf("%s.f%d", prefix, fi)}, fts[fi]))
		})
		return andAll(parts)
	case *ast.StructPat:
		v := fc.r.Variants[x.ID]
		fts := fc.g.fieldTypes(t, v.Index)
		var parts []string
		prefix := pl.s
		if t.Def.Adt.Kind == AdtEnum {
			parts = append(parts, fmt.Sprintf("%s.tag == %d", pl.s, v.Disc))
			prefix = fmt.Sprintf("%s.u.v%d", pl.s, v.Index)
		}
		for _, f := range x.Fields {
			fi := v.FieldIndex(f.Name.Name)
			parts = append(parts, fc.ptest(f.Pat, pplace{s: fmt.Sprintf("%s.f%d", prefix, fi)}, fts[fi]))
		}
		return andAll(parts)
	case *ast.PathPat:
		vr := fc.r.Paths[x.ID]
		return fc.pathTest(vr, pl, t)
	case *ast.RefPat:
		inner := t.Elem
		if inner.IsUnsized() {
			return fc.ptest(x.Pat, pplace{s: pl.s, unsized: true}, inner)
		}
		return fc.ptest(x.Pat, pplace{s: "(*" + pl.s + ")"}, inner)
	case *ast.BoxPat:
		inner := t.Args[0]
		return fc.ptest(x.Pat, pplace{s: "(*" + pl.s + ".f0)"}, inner)
	case *ast.SlicePat:
		return fc.sliceTest(x, pl, t)
	case *ast.OrPat:
		var parts []string
		for _, a := range x.Alts {
			parts = append(parts, fc.ptest(a, pl, t))
		}
		return "(" + strings.Join(parts, " || ") + ")"
	}
	fc.g.fatalf("%s: unsupported pattern in codegen", p.GetSpan().Lo)
	return ""
}

// seqFields maps tuple-like subpatterns (with optional `..`) to field indices.
func (fc *fnGen) seqFields(pats []ast.Pat, n int, f func(pi, fi int)) {
	rest := -1
	for i, p := range pats {
		if _, ok := p.(*ast.RestPat); ok {
			rest = i
		}
	}
	if rest < 0 {
		for i := range pats {
			f(i, i)
		}
		return
	}
	for i := 0; i < rest; i++ {
		f(i, i)
	}
	after := len(pats) - rest - 1
	for i := 0; i < after; i++ {
		f(rest+1+i, n-after+i)
	}
}

func (fc *fnGen) pathTest(vr *typeck.ValueRes, pl pplace, t Ty) string {
	switch vr.Kind {
	case typeck.VUnitCtor:
		if vr.Def.Kind == DefVariant {
			return fmt.Sprintf("(%s.tag == %d)", pl.s, vr.Def.Variant.Disc)
		}
		return "1"
	case typeck.VConst:
		d, args := vr.Def, vr.Args
		if d.InTrait() {
			d, args = fc.resolveAssocConst(d, args)
		}
		cv := fc.g.fnName(d, args) + "()"
		return fc.eqTest(pl, cv, t)
	}
	fc.g.fatalf("unsupported path pattern")
	return ""
}

func (fc *fnGen) eqTest(pl pplace, v string, t Ty) string {
	switch {
	case t.Kind == TRef && t.Elem.Kind == TStr:
		return fmt.Sprintf("rg_str_eq(%s, %s)", pl.s, v)
	case t.Kind == TStr:
		return fmt.Sprintf("rg_str_eq(%s, %s)", pl.s, v)
	case t.IsScalar():
		return fmt.Sprintf("(%s == %s)", pl.s, v)
	case t.Kind == TAdt && t.Def.Adt.Kind == AdtEnum:
		// fieldless enums compare by tag
		return fmt.Sprintf("(%s.tag == (%s).tag)", pl.s, v)
	}
	// fall back to PartialEq::eq
	eq := fc.g.tcx.Lang("eq")
	m := eq.Trait.Items["eq"]
	tmp := fc.spill(v, t)
	call := fc.callResolved(m, []Ty{t, t}, []string{"&" + pl.s, "&" + tmp}, TyBool)
	return call
}

func (fc *fnGen) litTest(e ast.Expr, pl pplace, t Ty) string {
	v := fc.value(e)
	return fc.eqTest(pl, v, t)
}

func (fc *fnGen) sliceTest(x *ast.SlicePat, pl pplace, t Ty) string {
	n := 0
	hasRest := false
	for _, e := range x.Elems {
		if isRestLike(e) {
			hasRest = true
		} else {
			n++
		}
	}
	var parts []string
	elem := t.Elem
	if t.Kind == TSlice {
		if hasRest {
			parts = append(parts, fmt.Sprintf("%s.meta >= %d", pl.s, n))
		} else {
			parts = append(parts, fmt.Sprintf("%s.meta == %d", pl.s, n))
		}
	}
	fc.sliceElems(x, pl, t, func(e ast.Pat, ep pplace) {
		parts = append(parts, fc.ptest(e, ep, elem))
	}, nil)
	return andAll(parts)
}

func isRestLike(p ast.Pat) bool {
	switch x := p.(type) {
	case *ast.RestPat:
		return true
	case *ast.IdentPat:
		_, ok := x.Sub.(*ast.RestPat)
		return ok
	}
	return false
}

// sliceElems visits element subpatterns (and optionally the rest binding).
func (fc *fnGen) sliceElems(x *ast.SlicePat, pl pplace, t Ty, f func(ast.Pat, pplace), rest func(ip *ast.IdentPat, start, after int)) {
	restIdx := -1
	for i, e := range x.Elems {
		if isRestLike(e) {
			restIdx = i
		}
	}
	elemAt := func(i int, fromEnd bool) pplace {
		if t.Kind == TArray {
			if fromEnd {
				return pplace{s: fmt.Sprintf("%s.a[%d]", pl.s, int(fc.g.arrayLen(t))-i)}
			}
			return pplace{s: fmt.Sprintf("%s.a[%d]", pl.s, i)}
		}
		ct := fc.ctype(t.Elem)
		if fromEnd {
			return pplace{s: fmt.Sprintf("((%s*)%s.ptr)[%s.meta - %d]", ct, pl.s, pl.s, i)}
		}
		return pplace{s: fmt.Sprintf("((%s*)%s.ptr)[%d]", ct, pl.s, i)}
	}
	if restIdx < 0 {
		for i, e := range x.Elems {
			f(e, elemAt(i, false))
		}
		return
	}
	for i := 0; i < restIdx; i++ {
		f(x.Elems[i], elemAt(i, false))
	}
	after := len(x.Elems) - restIdx - 1
	for i := 0; i < after; i++ {
		f(x.Elems[restIdx+1+i], elemAt(after-i, true))
	}
	if rest != nil {
		if ip, ok := x.Elems[restIdx].(*ast.IdentPat); ok {
			rest(ip, restIdx, after)
		}
	}
}

// ---------------------------------------------------------------------------
// Bindings

func (fc *fnGen) declarePatLocals(p ast.Pat) {
	ast.Walk(p, func(n any) bool {
		if ip, ok := n.(*ast.IdentPat); ok {
			if l := fc.r.Locals[ip.ID]; l != nil {
				fc.declLocal(l)
			}
		}
		return true
	})
}

// bindPat binds the pattern's variables from an owned place (no move
// tracking needed).
func (fc *fnGen) bindPat(p ast.Pat, place string, t Ty, owned bool) {
	fc.declarePatLocals(p)
	var moves []string
	fc.pbind(p, pplace{s: place, unsized: t.IsUnsized()}, t, &moves, false)
}

// bindPatPlace binds from a place operand, clearing its drop flag if a
// non-Copy value is moved out by a by-value binding.
func (fc *fnGen) bindPatPlace(p ast.Pat, place string, t Ty, op operand) []string {
	moves := fc.bindPatDeferred(p, place, t, op)
	for _, m := range moves {
		fc.line("%s", m)
	}
	return nil
}

// bindPatDeferred binds variables and returns move side effects (flag
// clears) to apply once the arm is committed (after guards).
func (fc *fnGen) bindPatDeferred(p ast.Pat, place string, t Ty, op operand) []string {
	fc.declarePatLocals(p)
	var moves []string
	moved := false
	fc.pbind(p, pplace{s: place, unsized: op.unsized || t.IsUnsized()}, t, &moves, false)
	if len(moves) > 0 {
		moved = true
	}
	var out []string
	if moved && op.flag != "" && op.movable {
		out = append(out, fmt.Sprintf("%s = 0;", op.flag))
	}
	return out
}

// pbind emits bindings; records in moves when a non-Copy value is moved.
func (fc *fnGen) pbind(p ast.Pat, pl pplace, t Ty, moves *[]string, inOr bool) {
	pl, t = fc.patDerefs(p, pl, t)
	switch x := p.(type) {
	case *ast.ParenPat:
		fc.pbind(x.Pat, pl, t, moves, inOr)
	case *ast.IdentPat:
		if fc.r.Paths[x.ID] != nil {
			return
		}
		l := fc.r.Locals[x.ID]
		if l == nil {
			return
		}
		if _, isRest := x.Sub.(*ast.RestPat); isRest {
			return // handled by slice binding
		}
		name := fc.localName(l)
		if l.ByRef {
			if pl.unsized {
				fc.line("%s = %s;", name, pl.s)
			} else {
				fc.line("%s = &%s;", name, pl.s)
			}
		} else if !isZST(l.Ty) {
			fc.line("%s = %s;", name, pl.s)
			if !fc.g.tcx.IsCopy(l.Ty) {
				*moves = append(*moves, name)
			}
		}
		fc.markInit(l)
		if x.Sub != nil {
			fc.pbind(x.Sub, pl, t, moves, inOr)
		}
	case *ast.TuplePat:
		fc.seqFields(x.Elems, len(t.Elems), func(pi, fi int) {
			fc.pbind(x.Elems[pi], pplace{s: fmt.Sprintf("%s.f%d", pl.s, fi)}, t.Elems[fi], moves, inOr)
		})
	case *ast.TupleStructPat:
		v := fc.r.Variants[x.ID]
		fts := fc.g.fieldTypes(t, v.Index)
		prefix := pl.s
		if t.Def.Adt.Kind == AdtEnum {
			prefix = fmt.Sprintf("%s.u.v%d", pl.s, v.Index)
		}
		fc.seqFields(x.Elems, len(fts), func(pi, fi int) {
			fc.pbind(x.Elems[pi], pplace{s: fmt.Sprintf("%s.f%d", prefix, fi)}, fts[fi], moves, inOr)
		})
	case *ast.StructPat:
		v := fc.r.Variants[x.ID]
		fts := fc.g.fieldTypes(t, v.Index)
		prefix := pl.s
		if t.Def.Adt.Kind == AdtEnum {
			prefix = fmt.Sprintf("%s.u.v%d", pl.s, v.Index)
		}
		for _, f := range x.Fields {
			fi := v.FieldIndex(f.Name.Name)
			fc.pbind(f.Pat, pplace{s: fmt.Sprintf("%s.f%d", prefix, fi)}, fts[fi], moves, inOr)
		}
	case *ast.RefPat:
		inner := t.Elem
		if inner.IsUnsized() {
			fc.pbind(x.Pat, pplace{s: pl.s, unsized: true}, inner, moves, inOr)
		} else {
			fc.pbind(x.Pat, pplace{s: "(*" + pl.s + ")"}, inner, moves, inOr)
		}
	case *ast.BoxPat:
		fc.pbind(x.Pat, pplace{s: "(*" + pl.s + ".f0)"}, t.Args[0], moves, inOr)
	case *ast.SlicePat:
		fc.sliceElems(x, pl, t, func(e ast.Pat, ep pplace) {
			fc.pbind(e, ep, t.Elem, moves, inOr)
		}, func(ip *ast.IdentPat, start, after int) {
			l := fc.r.Locals[ip.ID]
			if l == nil {
				return
			}
			name := fc.localName(l)
			ct := fc.ctype(t.Elem)
			if t.Kind == TArray {
				if l.ByRef {
					fc.line("%s = (%s)&%s.a[%d];", name, fc.ctype(l.Ty), pl.s, start)
				} else {
					fc.line("memcpy(&%s, &%s.a[%d], sizeof(%s));", name, pl.s, start, name)
				}
			} else {
				fc.line("%s = (Fat){ (void*)((%s*)%s.ptr + %d), %s.meta - %d };", name, ct, pl.s, start, pl.s, start+after)
			}
			fc.markInit(l)
		})
	case *ast.OrPat:
		if !patHasBindings(fc.r, x) {
			return
		}
		last := len(x.Alts) - 1
		for i, a := range x.Alts {
			switch {
			case i == 0:
				fc.line("if (%s) {", fc.ptest(a, pl, t))
			case i < last:
				fc.line("} else if (%s) {", fc.ptest(a, pl, t))
			default:
				fc.line("} else {")
			}
			fc.ind++
			fc.pbind(a, pl, t, moves, true)
			fc.ind--
		}
		fc.line("}")
	}
}

func patHasBindings(r *typeck.Results, p ast.Pat) bool {
	found := false
	ast.Walk(p, func(n any) bool {
		if ip, ok := n.(*ast.IdentPat); ok && r.Locals[ip.ID] != nil {
			found = true
		}
		return !found
	})
	return found
}
