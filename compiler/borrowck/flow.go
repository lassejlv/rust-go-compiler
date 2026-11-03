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
	tys := make([]Ty, len(args))
	for i, a := range args {
		tys[i] = a.Ty
	}
	fl := b.sigFlow(c, len(args), ret, tys)
	if !carriesBorrows(ret) {
		// only the stores into `&mut` arguments and 'static requirements
		return &Flow{Into: fl.Into, Static: fl.Static}
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
