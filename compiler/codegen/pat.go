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
			parts = append(parts, fc.g.enumIs(t, pl.s, v))
			prefix = fc.g.enumPayload(t, pl.s, v.Index)
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
			parts = append(parts, fc.g.enumIs(t, pl.s, v))
			prefix = fc.g.enumPayload(t, pl.s, v.Index)
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
			return fc.g.enumIs(t, pl.s, vr.Def.Variant)
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
		return fmt.Sprintf("(%s == %s)", fc.g.enumTag(t, pl.s), fc.g.enumTag(t, fc.spill(v, t)))
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
	var inits []string
	fc.deferInits = &inits
	fc.pbind(p, pplace{s: place, unsized: op.unsized || t.IsUnsized()}, t, &moves, false)
	fc.deferInits = nil
	if len(moves) > 0 {
		moved = true
	}
	out := inits
	if info, ok := fc.patSplit[p]; ok && moved && op.movable && place == fc.localName(info.local) {
		// whole fields moved out of a local: only their flags are cleared
		for _, i := range info.fields {
			out = append(out, fmt.Sprintf("%s = 0;", fc.pathFlags(info.local, []int{i})))
		}
		return out
	}
	if moved && op.flag != "" && op.movable {
		// The parts the pattern does not move out are still owned: schedule
		// their drops, armed (after a guard succeeds) from the source's drop
		// flag, which is cleared afterwards. With per-field flags (a local
		// partially moved or assigned before), each top-level field is armed
		// from its own flag.
		if fc.g.needsDrop(t) {
			flags := strings.Split(op.flag, " = ")
			base := flags[0]
			fieldFlag := map[string]string{}
			for _, f := range flags[1:] {
				if !strings.HasPrefix(f, base+"_") {
					continue
				}
				// df5_1_0: place.f1.f0 (place.a[1]... for arrays)
				k := place
				for j, n := range strings.Split(f[len(base)+1:], "_") {
					if j == 0 && t.Kind == TArray {
						k += ".a[" + n + "]"
					} else {
						k += ".f" + n
					}
				}
				fieldFlag[k] = f
			}
			fc.resSrcFlag = func(pl string) string {
				best, bestLen := base, -1
				for k, f := range fieldFlag {
					if (pl == k || strings.HasPrefix(pl, k+".") || strings.HasPrefix(pl, k+"[")) && len(k) > bestLen {
						best, bestLen = f, len(k)
					}
				}
				return best
			}
			sc := fc.scopes[len(fc.scopes)-1]
			mark := len(sc.drops)
			var act []string
			fc.resActivate = &act
			if parts, ok := fc.resParts[p]; ok && fc.patLocal(p, place) {
				// the parts left are dropped with the local: arm their
				// flags (declared with it)
				q := append([]resPart(nil), parts...)
				fc.resQueue = &q
			}
			fc.residualDrops(p, place, t, false)
			fc.resQueue = nil
			fc.resActivate = nil
			fc.resSrcFlag = nil
			out = append(out, act...)
			// fields are dropped in declaration order
			for i, j := mark, len(sc.drops)-1; i < j; i, j = i+1, j-1 {
				sc.drops[i], sc.drops[j] = sc.drops[j], sc.drops[i]
			}
			// the remainder belongs to the matched value, which is older
			// than the bindings: drop it after them
			if len(sc.drops) > mark {
				locals := map[*typeck.Local]bool{}
				ast.Walk(p, func(n any) bool {
					if ip, ok := n.(*ast.IdentPat); ok {
						if l := fc.r.Locals[ip.ID]; l != nil {
							locals[l] = true
						}
					}
					return true
				})
				first := mark
				for i := 0; i < mark; i++ {
					if sc.drops[i].local != nil && locals[sc.drops[i].local] {
						first = i
						break
					}
				}
				// a source local in this scope: the remainder takes its place
				for i := 0; i < mark; i++ {
					if sc.drops[i].local != nil && sc.drops[i].place == place {
						first = i + 1
						break
					}
				}
				if first < mark {
					res := append([]dropEntry(nil), sc.drops[mark:]...)
					rest := append([]dropEntry(nil), sc.drops[first:mark]...)
					sc.drops = append(append(sc.drops[:first], res...), rest...)
				}
			}
		}
		out = append(out, fmt.Sprintf("%s = 0;", op.flag))
	}
	return out
}

// patLocal reports whether place is the local whose residual parts were
// declared for p (see declResidualParts).
func (fc *fnGen) patLocal(p ast.Pat, place string) bool {
	for l, ps := range fc.resPats {
		for _, q := range ps {
			if q == p {
				return fc.declared[l] && place == fc.localName(l)
			}
		}
	}
	return false
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
		// or-pattern alternatives share the local but may bind it by value
		// in one alternative and by reference in another
		byRef := l.ByRef && t != l.Ty
		if byRef {
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
			prefix = fc.g.enumPayload(t, pl.s, v.Index)
		}
		fc.seqFields(x.Elems, len(fts), func(pi, fi int) {
			fc.pbind(x.Elems[pi], pplace{s: fmt.Sprintf("%s.f%d", prefix, fi)}, fts[fi], moves, inOr)
		})
	case *ast.StructPat:
		v := fc.r.Variants[x.ID]
		fts := fc.g.fieldTypes(t, v.Index)
		prefix := pl.s
		if t.Def.Adt.Kind == AdtEnum {
			prefix = fc.g.enumPayload(t, pl.s, v.Index)
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
					if !fc.g.tcx.IsCopy(l.Ty) {
						*moves = append(*moves, name)
					}
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

// patMoves reports whether binding p moves a non-Copy value out of the
// matched place.
func (fc *fnGen) patMoves(p ast.Pat) bool {
	moves := false
	ast.Walk(p, func(n any) bool {
		if ip, ok := n.(*ast.IdentPat); ok && fc.r.Paths[ip.ID] == nil {
			if l := fc.r.Locals[ip.ID]; l != nil && !l.ByRef && !isZST(l.Ty) && !fc.g.tcx.IsCopy(l.Ty) {
				moves = true
			}
		}
		return !moves
	})
	return moves
}

// patBorrows reports whether p binds anything by reference.
func (fc *fnGen) patBorrows(p ast.Pat) bool {
	refs := false
	ast.Walk(p, func(n any) bool {
		if ip, ok := n.(*ast.IdentPat); ok {
			if l := fc.r.Locals[ip.ID]; l != nil && l.ByRef {
				refs = true
			}
		}
		return !refs
	})
	return refs
}

// bindOwnedPat binds an irrefutable pattern from an owned place and takes
// care of the parts the pattern does not move out. With a local, its drop
// flag is cleared when parts were moved and the remainder is dropped at
// scope end; with now=true (a temporary), the remainder is dropped
// immediately unless the pattern borrows from it.
func (fc *fnGen) bindOwnedPat(p ast.Pat, place string, t Ty, l *typeck.Local, now bool) {
	sc := fc.scopes[len(fc.scopes)-1]
	mark := len(sc.drops)
	fc.declarePatLocals(p)
	var moves []string
	fc.pbind(p, pplace{s: place, unsized: t.IsUnsized()}, t, &moves, false)
	mid := len(sc.drops)
	// the remainder is dropped after the bindings (it was declared first)
	defer func() {
		if len(sc.drops) > mid {
			// residual parts are scheduled in drop order
			for i, j := mid, len(sc.drops)-1; i < j; i, j = i+1, j-1 {
				sc.drops[i], sc.drops[j] = sc.drops[j], sc.drops[i]
			}
			res := append([]dropEntry(nil), sc.drops[mid:]...)
			binds := append([]dropEntry(nil), sc.drops[mark:mid]...)
			sc.drops = append(append(sc.drops[:mark], res...), binds...)
		}
	}()
	if !fc.g.needsDrop(t) {
		return
	}
	if l != nil {
		if !fc.patMoves(p) {
			return // the local is dropped whole
		}
		fc.line("%s = 0;", fc.allFlags(l))
	}
	fc.residualDrops(p, place, t, now && !fc.patBorrows(p))
}

// residualDrops drops (now) or schedules the drop of every part of place
// that p does not move out.
func (fc *fnGen) residualDrops(p ast.Pat, place string, t Ty, now bool) {
	if !fc.g.needsDrop(t) {
		return
	}
	whole := func(pl string, ty Ty) {
		if !fc.g.needsDrop(ty) {
			return
		}
		if fc.resCollect != nil {
			*fc.resCollect = append(*fc.resCollect, resPart{place: pl, ty: ty})
			return
		}
		if fc.resQueue != nil && len(*fc.resQueue) > 0 && (*fc.resQueue)[0].place == pl {
			rp := (*fc.resQueue)[0]
			*fc.resQueue = (*fc.resQueue)[1:]
			src := "1"
			if fc.resSrcFlag != nil {
				src = fc.resSrcFlag(pl)
			}
			*fc.resActivate = append(*fc.resActivate, fmt.Sprintf("%s = %s;", rp.flag, src))
			return
		}
		if now {
			fc.line("%s(&%s);", fc.g.dropFn(ty), pl)
			return
		}
		flag := fc.newName("drf")
		if fc.resActivate != nil {
			// armed later, when the move out of the source is committed
			fc.line("uint8_t %s = 0;", flag)
			src := "1"
			if fc.resSrcFlag != nil {
				src = fc.resSrcFlag(pl)
			}
			*fc.resActivate = append(*fc.resActivate, fmt.Sprintf("%s = %s;", flag, src))
		} else {
			fc.line("uint8_t %s = 1;", flag)
		}
		fc.addDrop(pl, flag, ty)
		fc.line("RG_CS_PUSH(&%s, %s, &%s);", pl, fc.g.dropFn(ty), flag)
	}
	if p == nil || !fc.patMoves(p) {
		whole(place, t)
		return
	}
	fields := func(pats []ast.Pat, prefix string, fts []Ty, idx func(pi int) int, rest bool) {
		covered := map[int]bool{}
		if rest {
			fc.seqFields(pats, len(fts), func(pi, fi int) {
				covered[fi] = true
				fc.residualDrops(pats[pi], fmt.Sprintf("%s.f%d", prefix, fi), fts[fi], now)
			})
		} else {
			for pi := range pats {
				fi := idx(pi)
				covered[fi] = true
				fc.residualDrops(pats[pi], fmt.Sprintf("%s.f%d", prefix, fi), fts[fi], now)
			}
		}
		for fi, ft := range fts {
			if !covered[fi] {
				whole(fmt.Sprintf("%s.f%d", prefix, fi), ft)
			}
		}
	}
	switch x := p.(type) {
	case *ast.ParenPat:
		fc.residualDrops(x.Pat, place, t, now)
	case *ast.TuplePat:
		fields(x.Elems, place, t.Elems, nil, true)
	case *ast.TupleStructPat:
		v := fc.r.Variants[x.ID]
		prefix := place
		if t.Def.Adt.Kind == AdtEnum {
			prefix = fc.g.enumPayload(t, place, v.Index)
		}
		fields(x.Elems, prefix, fc.g.fieldTypes(t, v.Index), nil, true)
	case *ast.StructPat:
		v := fc.r.Variants[x.ID]
		prefix := place
		if t.Def.Adt.Kind == AdtEnum {
			prefix = fc.g.enumPayload(t, place, v.Index)
		}
		pats := make([]ast.Pat, len(x.Fields))
		for i, f := range x.Fields {
			pats[i] = f.Pat
		}
		fields(pats, prefix, fc.g.fieldTypes(t, v.Index), func(pi int) int { return v.FieldIndex(x.Fields[pi].Name.Name) }, false)
	case *ast.SlicePat:
		if t.Kind != TArray {
			return // elements cannot be moved out of a slice
		}
		fc.arrayResidualDrops(x, place, t, now)
	default:
		// by-value binding of the whole place (or unsupported shape): moved
	}
}

// arrayResidualDrops drops the elements of an array that a slice pattern
// does not move out, in rustc's order: the array is split into runs of
// untouched elements and elements with moves out of them, and these parts
// are dropped from the last to the first (each run front to back).
func (fc *fnGen) arrayResidualDrops(x *ast.SlicePat, place string, t Ty, now bool) {
	n := int(fc.g.arrayLen(t))
	pats := make([]ast.Pat, n) // element subpatterns (nil: none)
	moved := make([]bool, n)   // element (partly) moved out
	restIdx := -1
	for i, e := range x.Elems {
		if isRestLike(e) {
			restIdx = i
		}
	}
	for pi, e := range x.Elems {
		i := pi
		switch {
		case pi == restIdx:
			ip, ok := e.(*ast.IdentPat)
			if !ok {
				continue
			}
			if l := fc.r.Locals[ip.ID]; l != nil && !l.ByRef && !fc.g.tcx.IsCopy(l.Ty) {
				for j := restIdx; j < n-(len(x.Elems)-restIdx-1); j++ {
					moved[j] = true
				}
			}
			continue
		case restIdx >= 0 && pi > restIdx:
			i = n - (len(x.Elems) - pi)
		}
		if i >= 0 && i < n {
			pats[i] = e
			moved[i] = fc.patMoves(e)
		}
	}
	type part struct{ lo, hi int } // [lo, hi) untouched, or lo == hi-1 moved
	var parts []part
	var keep []bool
	for i := 0; i < n; {
		j := i + 1
		if !moved[i] {
			for j < n && !moved[j] {
				j++
			}
		}
		parts, keep = append(parts, part{i, j}), append(keep, moved[i])
		i = j
	}
	for k := len(parts) - 1; k >= 0; k-- {
		pr := parts[k]
		for i := pr.lo; i < pr.hi; i++ {
			el := fmt.Sprintf("%s.a[%d]", place, i)
			if keep[k] {
				if pats[i] != nil { // else moved into the rest binding
					fc.residualDrops(pats[i], el, t.Elem, now)
				}
			} else {
				fc.residualDrops(nil, el, t.Elem, now)
			}
		}
	}
}
