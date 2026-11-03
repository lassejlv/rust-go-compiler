package typeck

// Method resolution of generic bodies, reused by their instances.
//
// rustgo checks a generic body per instantiation, against concrete types,
// but rustc resolves its method calls once, against the bounds: with
// `x: &T` and no `T: Clone`, `x.clone()` is `<&T as Clone>::clone`
// (a `&T`), even when T = i32 has its own `clone`; with `T: Display`,
// `t.to_string()` is `ToString::to_string` even when the instance type
// has an inherent `to_string`. Resolving such calls on the concrete type
// would reject valid programs or silently call another method.
//
// genericPicks therefore type checks a generic body once with its own
// parameters as types (in the mode of the polymorphic body check, see
// genbody.go) and records, for every method call, the item it selected
// (inherent method, trait method of a bound, trait method through an
// impl) and how the receiver was adjusted (autoderef steps, autoref,
// array unsizing). Instances replay that pick (replayPick) with the
// container args substituted, and fall back to their own lookup when the
// pick does not apply. Picks are only used when the generic check of the
// whole body succeeded without any error: rustgo's polymorphic reasoning
// is weaker than rustc's, and a body it cannot check might have resolved
// a call differently than rustc does.

import (
	"fmt"
	"os"

	"rustgo/compiler/ast"
	"rustgo/compiler/lexer"
	. "rustgo/compiler/middle"
)

// genPick is a method call resolved in a generic body.
type genPick struct {
	item   *Def
	parent []Ty // container args in terms of the body's parameters (nil: unknown)
	step   int  // autoderef step of the receiver
	mode   int  // 0 by value, 1 `&` autoref, 2 `&mut` autoref
	unsize bool // array-to-slice unsizing after the last step
}

// genericPicks returns the method calls of def's generic body as resolved
// with its own parameters as types, or nil when they are not reliable.
func (t *Tcx) genericPicks(def *Def) map[ast.NodeID]*genPick {
	if t.genPicks == nil {
		t.genPicks = map[*Def]map[ast.NodeID]*genPick{}
	}
	if p, ok := t.genPicks[def]; ok {
		return p
	}
	t.genPicks[def] = nil // recursion guard
	// RUSTGO_GENPICKS=0: off; =nostd: not for std's generic bodies
	mode := os.Getenv("RUSTGO_GENPICKS")
	if mode == "0" {
		return nil
	}
	if def.Kind != DefFn || def.Fn == nil || def.Fn.Ast == nil || def.Fn.Ast.Body == nil || def.Crate == nil || def.Intrinsic != "" {
		return nil
	}
	if (mode == "nostd" && t.isStd(def)) || t.Generics(def).Count() == 0 {
		return nil
	}
	picks := t.computeGenericPicks(def)
	if os.Getenv("RUSTGO_TRACE_GENPICKS") != "" {
		if picks == nil {
			fmt.Fprintf(os.Stderr, "genpicks %s: unreliable\n", def.Path())
		} else {
			fmt.Fprintf(os.Stderr, "genpicks %s: %d calls\n", def.Path(), len(picks))
		}
	}
	t.genPicks[def] = picks
	return picks
}

// computeGenericPicks runs the generic check of def (see genericPicks).
func (t *Tcx) computeGenericPicks(def *Def) (picks map[ast.NodeID]*genPick) {
	ps := &polyState{def: def}
	saved := t.poly
	t.genPickOuter = append(t.genPickOuter, genPickOuter{quiet: t.quiet, poly: saved})
	n := len(t.genPickOuter)
	t.poly = ps
	t.quiet++
	defer func() {
		t.quiet--
		t.poly = saved
		t.genPickOuter = t.genPickOuter[:n-1]
		if r := recover(); r != nil {
			if _, fatal := r.(fatalError); fatal {
				// too many errors (of a concrete instance checked meanwhile)
				panic(r)
			}
			picks = nil
		}
	}()
	fcx := t.newFnCtxt(def, t.Generics(def).Identity())
	fcx.recPicks = map[ast.NodeID]*genPick{}
	fcx.checkFn()
	if len(ps.errs) > 0 {
		return nil
	}
	for _, p := range fcx.recPicks {
		for i, a := range p.parent {
			a = fcx.ic.resolve(a)
			if a.HasInfer() || a.HasError() || a.HasOpaque() || mentionsClosure(a) {
				a = nil
			}
			p.parent[i] = a
		}
	}
	return fcx.recPicks
}

// genPickOuter is the error mode outside a genericPicks check.
type genPickOuter struct {
	quiet int
	poly  *polyState
}

func anyHasParam(args []Ty) bool {
	for _, a := range args {
		if a != nil && a.HasParam() {
			return true
		}
	}
	return false
}

// mentionsClosure reports whether x contains a closure type.
func mentionsClosure(x Ty) bool {
	found := false
	Walk(x, func(y Ty) bool {
		if y.Kind == TClosure {
			found = true
		}
		return !found
	})
	return found
}

// recordPick records the pick of a method call during a generic check.
func (fcx *FnCtxt) recordPick(id ast.NodeID, p *pick) {
	if fcx.recPicks == nil {
		return
	}
	if !p.probed {
		// e.g. through a pending `impl Trait`: nothing to replay
		delete(fcx.recPicks, id)
		return
	}
	fcx.recPicks[id] = &genPick{item: p.item, parent: append([]Ty(nil), p.parent...), step: p.step, mode: p.mode, unsize: p.unsize}
}

// instPick returns the generic body's pick of the method call id when fcx
// checks an instance of a generic body.
func (fcx *FnCtxt) instPick(id ast.NodeID) *genPick {
	if fcx.recPicks != nil || fcx.def == nil || fcx.inst.Def != fcx.def || len(fcx.inst.Args) == 0 {
		return nil
	}
	for _, a := range fcx.inst.Args {
		if a == nil || a.HasParam() || a.HasInfer() || a.HasError() {
			return nil
		}
	}
	return fcx.t.genericPicks(fcx.def)[id]
}

// replayPick applies the generic body's pick gp to the receiver type rt of
// an instance: the same item with the same receiver adjustments. It
// returns nil (with nothing committed) when the pick does not apply.
func (fcx *FnCtxt) replayPick(rt Ty, gp *genPick, sp lexer.Span) *pick {
	item := gp.item
	inherent := item.InImpl() && item.Parent.Impl != nil && item.Parent.Impl.TraitRef == nil
	if !item.InTrait() && !inherent {
		return nil
	}
	steps := fcx.autoderef(rt, sp)
	si := gp.step
	last := si
	if gp.unsize {
		// the array is the last autoderef step, then a slice
		if si != len(steps) || si == 0 || steps[si-1].ty.Kind != TArray || gp.mode == 0 {
			return nil
		}
		steps = append(steps, autoderefStep{ty: Slice(steps[si-1].ty.Elem)})
		last = si - 1
	} else if si >= len(steps) {
		return nil
	}
	for k := 0; k <= si; k++ {
		if steps[k].ty.Kind == TInfer || steps[k].ty.Kind == TError {
			return nil
		}
	}
	u := steps[si].ty
	adjusted := u
	if gp.mode > 0 {
		adjusted = Ref(u, gp.mode == 2)
	}
	adj := adjustmentsFor(steps, last)
	if gp.mode > 0 {
		adj = append(adj, Adjustment{Kind: AdjBorrow, Mut: gp.mode == 2, Target: Ref(steps[last].ty, gp.mode == 2)})
		if gp.unsize {
			adj = append(adj, Adjustment{Kind: AdjUnsize, Target: adjusted})
		}
	}
	s := fcx.ic.snapshot()
	if inherent {
		g := fcx.t.Generics(item.Parent)
		iargs := make([]Ty, len(g.Params))
		for i := range iargs {
			iargs[i] = fcx.ic.newVar()
		}
		if !fcx.ic.unify(fcx.methodSelfTy(item, iargs), adjusted) {
			fcx.ic.rollback(s)
			return nil
		}
		return &pick{item: item, parent: iargs, adjust: adj, recvTy: adjusted, step: si, mode: gp.mode, unsize: gp.unsize, probed: true}
	}
	tr := item.Parent
	n := len(fcx.t.Generics(tr).Params)
	targs := make([]Ty, n)
	for i := range targs {
		if i > 0 && i < len(gp.parent) && gp.parent[i] != nil {
			targs[i] = fcx.normalizeInBody(Subst(gp.parent[i], fcx.inst.Args), sp)
		} else {
			targs[i] = fcx.ic.newVar()
		}
	}
	if !fcx.ic.unify(fcx.methodSelfTy(item, targs), adjusted) {
		fcx.ic.rollback(s)
		return nil
	}
	if st := fcx.shallow(targs[0]); st.Kind == TInfer && st.Sub == 0 {
		fcx.ic.rollback(s)
		return nil
	}
	ref := &TraitRef{Trait: tr, Args: targs}
	if fcx.ic.evaluate(&Predicate{Trait: ref}, 0) == SelNone {
		fcx.ic.rollback(s)
		return nil
	}
	fcx.ic.obligations = append(fcx.ic.obligations, &Obligation{Pred: &Predicate{Trait: ref}, Span: sp})
	p := &pick{item: item, parent: targs, adjust: adj, recvTy: adjusted, step: si, mode: gp.mode, unsize: gp.unsize, probed: true}
	if st := fcx.shallow(targs[0]); st.Kind == TDyn && fcx.t.IsObjectMethod(item, st) {
		p.virtual = true
	}
	return p
}
