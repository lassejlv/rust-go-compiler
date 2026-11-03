package borrowck

import (
	"strings"

	"rustgo/compiler/ast"
	. "rustgo/compiler/middle"
)

// boundCallSlots makes the flow of a call of a closure the body received
// through a type parameter (`F: FnMut(&I::Item) -> K`, called on the
// instance's F) follow the Fn* bound: the regions the bound's inputs
// declare higher-ranked (elided, or bound by `for<'r>`) and its output
// does not mention cannot flow into the result. The other slots keep the
// flow by depth.
func (b *builder) boundCallSlots(fl *Flow, ct Ty, callee Place, args []Operand, ret Ty) {
	if ret == nil || b.r == nil || b.r.Inst.Def == nil || len(args) < 2 {
		return
	}
	inputs, output, hr, ok := b.fnBoundOf(ct)
	params := map[string]bool{}
	for _, g := range astGenerics(b.r.Inst.Def) {
		genericNames(g, params)
	}
	if !ok {
		// a parameter (or a field of one) declared `dyn Fn(&T) -> K`
		// or `impl Fn(&T) -> K`
		inputs, output, hr, params, ok = b.declaredFnSig(callee)
	}
	if !ok || len(inputs) != len(args)-1 {
		return
	}
	rl := layoutOf(ret)
	if rl.flat {
		return
	}
	tys := b.argTypes(args)
	w := newLabeler(params, nil)
	labels := make([][]string, len(args))
	lays := make([]*layout, len(args))
	for i, in := range inputs {
		t := tys[i+1]
		if t == nil {
			return
		}
		l := layoutOf(t)
		if l.flat {
			return
		}
		w.label(in, l)
		labels[i+1] = w.labels
		lays[i+1] = l
	}
	nElided := w.elided + w.hidden
	// the lifetimes the output mentions ("'?": an elided one)
	outNames := map[string]bool{}
	sw := &sigWalker{params: params, inOut: true}
	sw.walk(output, 0)
	for _, o := range sw.occs {
		outNames[o.name] = true
	}
	if sw.implAll {
		return
	}
	ow := newLabeler(params, nil)
	ow.out = true
	ow.label(output, rl)
	for _, lb := range ow.labels {
		outNames[lb] = true
	}
	if outNames["'?"] && nElided > 1 {
		return
	}
	excluded := false
	ex := make([][]bool, len(args))
	for i := 1; i < len(args); i++ {
		ex[i] = make([]bool, lays[i].n())
		for k, lb := range labels[i] {
			if lb == "" || lb[0] != '\'' || outNames[lb] {
				continue
			}
			elided := strings.HasPrefix(lb, "'#")
			if elided && outNames["'?"] {
				continue // the output's elided lifetime is this one
			}
			if elided || hr[lb] {
				ex[i][k] = true
				excluded = true
			}
		}
	}
	if !excluded {
		return
	}
	// the flow by depth (each argument's depth d into the result's depth
	// d, see callExpr) without the excluded slots
	fl.RetLay = rl
	fl.ArgLays = make([]*layout, len(args))
	fl.ArgFull = make([]bool, len(args))
	fl.Covered = make([]bool, rl.n())
	for j := range fl.Covered {
		fl.Covered[j] = true
	}
	for i := range args {
		fl.ArgFull[i] = true
		if i == 0 {
			continue // the callee: nothing flows from it (see callExpr)
		}
		fl.ArgLays[i] = lays[i]
		for k, dk := range lays[i].depth {
			if ex[i][k] {
				continue
			}
			for j, dj := range rl.depth {
				if dj == dk {
					fl.Slots = append(fl.Slots, SlotEdge{Arg: i, From: k, To: j})
				}
			}
		}
	}
}

// astGenerics returns the generics of a function and of its impl or
// trait.
func astGenerics(d *Def) []*ast.Generics {
	if d == nil || d.Fn == nil || d.Fn.Ast == nil {
		return nil
	}
	gens := []*ast.Generics{d.Fn.Ast.Generics}
	if p := d.Parent; p != nil && p.Item != nil {
		switch k := p.Item.Kind.(type) {
		case *ast.ImplItem:
			gens = append(gens, k.Generics)
		case *ast.TraitItem:
			gens = append(gens, k.Generics)
		}
	}
	return gens
}

// fnBoundOf finds the Fn* bound of the type parameter of the function
// being checked whose instance is ct: its inputs, output and the
// lifetimes it binds (`for<'r>`). ok is false unless exactly one
// parameter is instantiated with ct and it has exactly one Fn* bound.
func (b *builder) fnBoundOf(ct Ty) (inputs []ast.Type, output ast.Type, hr map[string]bool, ok bool) {
	if ct == nil {
		return nil, nil, nil, false
	}
	d := b.r.Inst.Def
	g := b.tcx.Generics(d)
	if g == nil || len(g.Params) != len(b.r.Inst.Args) {
		return nil, nil, nil, false
	}
	name := ""
	for k, a := range b.r.Inst.Args {
		if a != ct {
			continue
		}
		p := g.Params[k]
		if name != "" || p.Const || p.Synthetic {
			return nil, nil, nil, false
		}
		name = p.Name
	}
	if name == "" {
		return nil, nil, nil, false
	}
	n := 0
	visit := func(fors []string, bs []*ast.Bound) {
		for _, bd := range bs {
			if bd.Path == nil || len(bd.Path.Segments) == 0 {
				continue
			}
			seg := bd.Path.Segments[len(bd.Path.Segments)-1]
			switch seg.Ident.Name {
			case "Fn", "FnMut", "FnOnce":
			default:
				continue
			}
			n++
			if seg.Args == nil || !seg.Args.Parenthesized {
				ok = false
				continue
			}
			inputs, output = seg.Args.Inputs, seg.Args.Output
			hr = map[string]bool{}
			for _, l := range fors {
				hr[l] = true
			}
			for _, l := range bd.ForLifetimes {
				hr[l] = true
			}
			ok = true
		}
	}
	for _, gs := range astGenerics(d) {
		if gs == nil {
			continue
		}
		for _, gp := range gs.Params {
			if gp.Kind == ast.GPType && gp.Name.Name == name {
				visit(nil, gp.Bounds)
			}
		}
		for _, wp := range gs.Where {
			pt, isPath := wp.Ty.(*ast.PathType)
			if !isPath || pt.QSelf != nil || pt.Path == nil || len(pt.Path.Segments) != 1 || pt.Path.Segments[0].Ident.Name != name || pt.Path.Segments[0].Args != nil {
				continue
			}
			visit(wp.ForLifetimes, wp.Bounds)
		}
	}
	if n != 1 {
		return nil, nil, nil, false
	}
	return inputs, output, hr, ok
}
