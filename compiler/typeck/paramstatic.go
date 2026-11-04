package typeck

// E0310: a value whose type holds a type parameter is unsized into a trait
// object that must be 'static (a trait with a `'static` supertrait such as
// `Any`, or the `Box<dyn Trait>` a function returns, whose default object
// lifetime is 'static) while the parameter is not bounded by 'static.
// Types carry no lifetimes in rustgo, so only these syntactic cases are
// checked.

import (
	"rustgo/compiler/ast"
	. "rustgo/compiler/middle"
)

func (t *Tcx) checkParamStatic(r *Results, def *Def) {
	if def == nil || def.Crate == nil || !def.Crate.Local || def.Kind != DefFn || def.Fn == nil || def.Fn.Ast == nil || def.Fn.Ast.Body == nil {
		return
	}
	g := t.Generics(def)
	if g == nil || len(g.Params) == 0 {
		return
	}
	returned := map[ast.NodeID]bool{}
	if retStaticObject(def.Fn.Ast.Decl.Output) {
		markReturned(def.Fn.Ast.Body, returned)
	}
	reported := false
	ast.Walk(def.Fn.Ast.Body, func(n any) bool {
		if reported {
			return false
		}
		switch n.(type) {
		case *ast.ItemStmt, *ast.ClosureExpr:
			return false
		}
		e, ok := n.(ast.Expr)
		if !ok {
			return true
		}
		adj := r.Adjust[e.GetID()]
		src := r.NodeTypes[e.GetID()]
		for _, a := range adj {
			if a.Kind == AdjUnsize && src != nil && a.Target != nil {
				dyn := firstDyn(a.Target)
				if dyn != nil && (returned[e.GetID()] || t.traitRequiresStatic(dyn.Def, 0)) {
					if p := t.unboundedParam(g, src); p != nil {
						t.ErrorCode(e.GetSpan(), "E0310", "the parameter type `%s` may not live long enough", p.Name)
						reported = true
						return false
					}
				}
			}
			if a.Target != nil {
				src = a.Target
			}
		}
		return true
	})
}

// retStaticObject reports whether a declared return type is `Box<dyn
// Trait>` (default object lifetime 'static) or `Box<dyn Trait + 'static>`.
func retStaticObject(out ast.Type) bool {
	pt, ok := out.(*ast.PathType)
	if !ok || pt.QSelf != nil || pt.Path == nil || len(pt.Path.Segments) == 0 {
		return false
	}
	seg := pt.Path.Segments[len(pt.Path.Segments)-1]
	if seg.Ident.Name != "Box" || seg.Args == nil || len(seg.Args.Args) != 1 || seg.Args.Args[0].Type == nil {
		return false
	}
	dt, ok := seg.Args.Args[0].Type.(*ast.DynTraitType)
	if !ok {
		return false
	}
	for _, b := range dt.Bounds {
		if b.Lifetime != "" && b.Lifetime != "'static" {
			return false
		}
		if b.Path != nil {
			for _, s := range b.Path.Segments {
				if s.Args != nil {
					for _, ga := range s.Args.Args {
						if ga.Lifetime != "" {
							return false
						}
					}
				}
			}
		}
	}
	return true
}

// markReturned records the expressions a function body returns: its tail
// and `return` operands (through blocks, ifs and matches).
func markReturned(b *ast.Block, out map[ast.NodeID]bool) {
	var tail func(e ast.Expr)
	tail = func(e ast.Expr) {
		if e == nil {
			return
		}
		out[e.GetID()] = true
		switch x := e.(type) {
		case *ast.ParenExpr:
			tail(x.X)
		case *ast.BlockExpr:
			if !x.Async && !x.Const {
				retBlockTail(x.Block, tail)
			}
		case *ast.IfExpr:
			retBlockTail(x.Then, tail)
			tail(x.Else)
		case *ast.MatchExpr:
			for _, a := range x.Arms {
				tail(a.Body)
			}
		}
	}
	retBlockTail(b, tail)
	ast.Walk(b, func(n any) bool {
		switch x := n.(type) {
		case *ast.ItemStmt, *ast.ClosureExpr:
			return false
		case *ast.BlockExpr:
			if x.Async {
				return false
			}
		case *ast.ReturnExpr:
			tail(x.Value)
		}
		return true
	})
}

func retBlockTail(b *ast.Block, f func(ast.Expr)) {
	if b == nil || len(b.Stmts) == 0 {
		return
	}
	if es, ok := b.Stmts[len(b.Stmts)-1].(*ast.ExprStmt); ok && !es.Semi {
		f(es.Expr)
	}
}

func firstDyn(t Ty) Ty {
	var d Ty
	Walk(t, func(x Ty) bool {
		if d != nil {
			return false
		}
		if x.Kind == TDyn && x.Def != nil {
			d = x
			return false
		}
		return true
	})
	return d
}

// traitRequiresStatic reports whether a trait has a `'static` supertrait
// bound (directly or through its supertraits).
func (t *Tcx) traitRequiresStatic(tr *Def, depth int) bool {
	if tr == nil || tr.Item == nil || depth > 8 {
		return false
	}
	ti, ok := tr.Item.Kind.(*ast.TraitItem)
	if !ok {
		return false
	}
	for _, b := range ti.Supertraits {
		if b.Lifetime == "'static" {
			return true
		}
	}
	var sups []*TraitRef
	if !t.quietly(func() {
		sups = t.AllSupertraits(&TraitRef{Trait: tr, Args: t.Generics(tr).Identity()})
	}) {
		return false
	}
	for _, s := range sups {
		if s.Trait != tr && s.Trait != nil && s.Trait.Item != nil {
			if si, ok := s.Trait.Item.Kind.(*ast.TraitItem); ok {
				for _, b := range si.Supertraits {
					if b.Lifetime == "'static" {
						return true
					}
				}
			}
		}
	}
	return false
}

// unboundedParam returns a type parameter in ty that is not bounded by
// 'static (nil if none).
func (t *Tcx) unboundedParam(g *Generics, ty Ty) *GenericParamDef {
	var found *GenericParamDef
	Walk(ty, func(x Ty) bool {
		if found != nil {
			return false
		}
		switch x.Kind {
		case TRef, TFnPtr, TDyn, TClosure, TOpaque, TProjection, TFnDef:
			// references bring their own lifetimes; leave other
			// structural types alone
			return false
		case TParam:
			if x.Sub != 0 || x.Index >= len(g.Params) {
				return false
			}
			gp := g.Params[x.Index]
			if gp.Const || gp.Name == "Self" || gp.Synthetic || gp.Ast == nil {
				return false
			}
			if !t.paramStatic(g, gp, x) {
				found = gp
			}
			return false
		}
		return true
	})
	return found
}

func (t *Tcx) paramStatic(g *Generics, gp *GenericParamDef, pt Ty) bool {
	for _, b := range gp.Ast.Bounds {
		if b.Lifetime == "'static" {
			return true
		}
	}
	for d := gp.Owner; d != nil; d = d.Parent {
		ga := genericsAst(d)
		if ga != nil {
			for _, w := range ga.Where {
				p, ok := w.Ty.(*ast.PathType)
				if !ok || p.QSelf != nil || p.Path == nil || len(p.Path.Segments) != 1 || p.Path.Segments[0].Ident.Name != gp.Name {
					continue
				}
				for _, b := range w.Bounds {
					if b.Lifetime == "'static" {
						return true
					}
				}
			}
		}
		if d.Kind == DefCrate || d.Kind == DefMod {
			break
		}
	}
	for _, p := range g.Predicates {
		if p.Maybe || p.Trait == nil || len(p.Trait.Args) == 0 || p.Trait.Args[0] != pt {
			continue
		}
		if t.traitRequiresStatic(p.Trait.Trait, 0) {
			return true
		}
	}
	return false
}
