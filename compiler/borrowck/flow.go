package borrowck

import (
	"rustgo/compiler/ast"
	. "rustgo/compiler/middle"
	"rustgo/compiler/typeck"
)

// FlowEdge: result.loans[To+k] ⊇ arg[Arg].loans[From+k] for k = 0, or for
// all k >= 0 when Deep.
type FlowEdge struct {
	Arg, From, To int
	Deep          bool
}

// Flow describes how loans flow through a call.
type Flow struct {
	Edges []FlowEdge
	// All: the result may hold any loan of any argument (at its depth).
	All bool
	// Into: arguments stored into the referents of `&mut` arguments.
	Into []IntoEdge
	// Static: arguments that must not borrow locals ('static).
	Static []StaticReq
	// Precise flows by region slot (see slots.go), for arguments and a
	// result with layouts ArgLays and RetLay: Slots feed the Covered
	// result slots from the arguments with ArgFull (the others by depth).
	Slots   []SlotEdge
	Covered []bool
	ArgFull []bool
	RetLay  *layout
	ArgLays []*layout
	// ArgLabels: the labels of the arguments' slots in the signature
	// (sigslots.go), where computed
	ArgLabels [][]string
	// IntoSlots: slots of arguments stored into slots of the referent of
	// a `&mut` argument (DstSlot in the argument's layout).
	IntoSlots []IntoSlot
}

// SlotEdge: result slot To receives the loans of slot From of argument Arg.
type SlotEdge struct {
	Arg, From, To int
}

type IntoSlot struct {
	Dst, DstSlot, Src, SrcSlot int
}

func noFlow() *Flow { return &Flow{} }

// refFlow: the result reborrows from the first argument (deref, index).
func refFlow() *Flow { return &Flow{Edges: []FlowEdge{{Arg: 0, From: 0, To: 0, Deep: true}}} }

func opaqueFlow(t Ty) *Flow {
	if carriesBorrows(t) {
		return &Flow{All: true}
	}
	return noFlow()
}

// carriesBorrows reports whether values of t may hold references.
func carriesBorrows(t Ty) bool {
	if t == nil {
		return false
	}
	switch t.Kind {
	case TRef, TDyn, TClosure, TOpaque, TParam, TProjection, TInfer, TError:
		return true
	case TPtr, TInt, TFloat, TBool, TChar, TNever, TFnPtr, TFnDef, TStr:
		return false
	case TArray, TSlice:
		return carriesBorrows(t.Elem)
	case TTuple:
		for _, e := range t.Elems {
			if carriesBorrows(e) {
				return true
			}
		}
		return false
	case TAdt:
		for _, a := range t.Args {
			if carriesBorrows(a) {
				return true
			}
		}
		return adtHasLifetimes(t.Def)
	}
	return true
}

// flowFor computes the loan flow of a resolved call.
func (b *builder) flowFor(c *typeck.Callee, args []Operand, ret Ty) *Flow {
	if c == nil {
		return noFlow()
	}
	tys := b.argTypes(args)
	fl := b.sigFlow(c, len(args), ret, tys)
	if !carriesBorrows(ret) {
		// only the stores into `&mut` arguments and 'static requirements
		return &Flow{Into: fl.Into, Static: fl.Static, IntoSlots: fl.IntoSlots, ArgLays: fl.ArgLays, ArgFull: fl.ArgFull, RetLay: fl.RetLay, Covered: fl.Covered, ArgLabels: fl.ArgLabels}
	}
	return fl
}

// adtHasLifetimes reports whether an ADT declares lifetime parameters.
func adtHasLifetimes(d *Def) bool {
	if d == nil || d.Item == nil {
		return false
	}
	var g *ast.Generics
	switch k := d.Item.Kind.(type) {
	case *ast.StructItem:
		g = k.Generics
	case *ast.EnumItem:
		g = k.Generics
	case *ast.UnionItem:
		g = k.Generics
	}
	if g == nil {
		return false
	}
	for _, p := range g.Params {
		if p.Kind == ast.GPLifetime {
			return true
		}
	}
	return false
}

// argTypes returns the types of call operands (nil: unknown).
func (b *builder) argTypes(args []Operand) []Ty {
	tys := make([]Ty, len(args))
	for i, a := range args {
		tys[i] = a.Ty
		if a.Ty == nil && a.Kind != OConst && len(a.Place.Proj) == 0 && int(a.Place.Local) < len(b.body.Locals) {
			tys[i] = b.body.Locals[a.Place.Local].Ty
		}
		if a.Ty == nil && a.Kind == OConst {
			tys[i] = Tuple(nil) // holds no loans
		}
	}
	return tys
}

// projSlots makes the flow of an overloaded index or deref (`&B -> &X`)
// precise: the result's target is the part of B of X's layout (Vec<T> ->
// T, Box<T> -> T); deref: the target also lives as long as B's own
// references (Ref<'b, T>, Pin<&mut T>).
func projSlots(fl *Flow, argTys []Ty, ret Ty, deref bool) {
	if len(argTys) == 0 || ret == nil || ret.Kind != TRef {
		return
	}
	for _, t := range argTys {
		if t == nil {
			return
		}
	}
	al := layoutOf(argTys[0])
	rl := layoutOf(ret)
	if al.flat || rl.flat || al.root.ty == nil || al.root.ty.Kind != TRef {
		return
	}
	b, x := al.root.kids[0], rl.root.kids[0]
	matches := findKey(b, layoutOf(x.ty).key)
	if len(matches) == 0 {
		return
	}
	fl.RetLay = rl
	for _, t := range argTys {
		fl.ArgLays = append(fl.ArgLays, layoutOf(t))
		fl.ArgFull = append(fl.ArgFull, true)
	}
	fl.Covered = make([]bool, rl.n())
	for j := range fl.Covered {
		fl.Covered[j] = true
	}
	fl.Slots = append(fl.Slots, SlotEdge{Arg: 0, From: 0, To: 0})
	if deref {
		for k := b.start; k < b.end; k++ {
			if al.depth[k] == b.depth {
				fl.Slots = append(fl.Slots, SlotEdge{Arg: 0, From: k, To: 0})
			}
		}
	}
	for _, m := range matches {
		for k := 0; k < x.end-x.start; k++ {
			fl.Slots = append(fl.Slots, SlotEdge{Arg: 0, From: m.start + k, To: x.start + k})
		}
	}
}

// findKey returns the outermost nodes of the subtree n with slot layout
// key (none for layouts without slots).
func findKey(n *lnode, key string) []*lnode {
	if n == nil || key == "" || n.ty == nil {
		return nil
	}
	if layoutOf(n.ty).key == key {
		return []*lnode{n}
	}
	var out []*lnode
	for _, k := range n.kids {
		out = append(out, findKey(k, key)...)
	}
	return out
}

// refsBesidesDyn reports whether values of t may hold references other
// than trait objects not behind references.
func refsBesidesDyn(t Ty) bool {
	if t == nil {
		return true
	}
	switch t.Kind {
	case TDyn:
		return false
	case TArray, TSlice:
		return refsBesidesDyn(t.Elem)
	case TTuple:
		for _, e := range t.Elems {
			if refsBesidesDyn(e) {
				return true
			}
		}
		return false
	case TAdt:
		if adtHasLifetimes(t.Def) {
			return true
		}
		for _, a := range t.Args {
			if refsBesidesDyn(a) {
				return true
			}
		}
		return false
	}
	return carriesBorrows(t)
}

// closureCallSlots makes the flow of a call of a closure of this body
// precise from its summary: each result slot holds the argument slots the
// closure's result is made of (the result never borrows from the closure
// itself; see the call's flow).
func (b *builder) closureCallSlots(fl *Flow, ct Ty, args []Operand, ret Ty) {
	sm := closSummaries[ct]
	if sm == nil || sm.ret == nil || sm.ret.flat || sm.imprecise || ret == nil || len(args) != len(sm.params)+1 {
		return
	}
	local := false
	for _, ci := range b.r.Closures {
		if ci.Ty == ct {
			local = true
			break
		}
	}
	rl := layoutOf(ret)
	if !local || rl.flat || rl.key != sm.ret.key || rl.n() != sm.ret.n() {
		return
	}
	tys := b.argTypes(args)
	for k, pl := range sm.params {
		al := layoutOf(tys[k+1])
		if pl.flat || (args[k+1].Kind != OConst && (al.key != pl.key || al.n() != pl.n())) {
			return
		}
	}
	fl.RetLay = rl
	fl.ArgLays = append([]*layout{nil}, sm.params...) // nil: the callee
	fl.ArgFull = make([]bool, len(args))
	for i := range fl.ArgFull {
		fl.ArgFull[i] = true
	}
	fl.Covered = make([]bool, rl.n())
	for j := range fl.Covered {
		fl.Covered[j] = true
	}
	for j, srcs := range sm.fromArg {
		for _, a := range srcs {
			fl.Slots = append(fl.Slots, SlotEdge{Arg: a[0] + 1, From: a[1], To: j})
		}
	}
}
