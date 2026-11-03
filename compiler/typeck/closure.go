package typeck

import (
	"rustgo/compiler/ast"
	"rustgo/compiler/lexer"
	. "rustgo/compiler/middle"
)

// expectedClosureSig deduces a closure signature from the expected type.
func (fcx *FnCtxt) expectedClosureSig(expected Ty) ([]Ty, Ty) {
	if expected == nil {
		return nil, nil
	}
	et := fcx.shallow(expected)
	switch et.Kind {
	case TFnPtr:
		return et.Elems, et.Ret
	case TDyn:
		if et.Def != nil && (et.Def.LangItem == "fn" || et.Def.LangItem == "fn_mut" || et.Def.LangItem == "fn_once") && len(et.Args) > 0 {
			var out Ty
			for _, b := range et.Bindings {
				if b.Name == "Output" {
					out = b.Ty
				}
			}
			if a := fcx.shallow(et.Args[0]); a.Kind == TTuple {
				return a.Elems, out
			}
		}
	case TRef:
		return fcx.expectedClosureSig(et.Elem)
	case TAdt:
		if et.Def.LangItem == "owned_box" {
			return fcx.expectedClosureSig(et.Args[0])
		}
	case TInfer:
		var ins []Ty
		var out Ty
		for _, o := range fcx.ic.obligations {
			if o.Pred == nil {
				continue
			}
			tr := o.Pred.Trait
			ln := tr.Trait.LangItem
			if ln != "fn" && ln != "fn_mut" && ln != "fn_once" {
				continue
			}
			if fcx.shallow(tr.Args[0]) != et {
				continue
			}
			if len(tr.Args) > 1 {
				a := fcx.structurally(tr.Args[1])
				if a.Kind == TTuple && ins == nil {
					ins = a.Elems
				}
			}
			for _, b := range o.Pred.Bindings {
				if b.Name == "Output" && out == nil {
					out = b.Ty
				}
			}
		}
		// obligations from impl where-clauses may carry projections
		// (`&<I as Iterator>::Item`): normalize what is known now
		sp := lexerSpanZero
		if ins != nil {
			nins := make([]Ty, len(ins))
			for i, t := range ins {
				nins[i] = fcx.normalizeInBody(t, sp)
			}
			ins = nins
		}
		if out != nil {
			out = fcx.normalizeInBody(out, sp)
		}
		return ins, out
	}
	return nil, nil
}

func (fcx *FnCtxt) checkClosure(x *ast.ClosureExpr, expected Ty) Ty {
	expIns, expOut := fcx.expectedClosureSig(expected)
	if h := fcx.closureHints[x.ID]; h != nil && expIns == nil && expOut == nil {
		expIns, expOut = fcx.expectedClosureSig(h)
	}
	ins := make([]Ty, len(x.Params))
	for i, p := range x.Params {
		switch {
		case p.Ty != nil:
			ins[i] = fcx.lowerTy(p.Ty)
			if i < len(expIns) {
				fcx.ic.unify(ins[i], expIns[i])
			}
		case i < len(expIns) && len(expIns) == len(x.Params):
			ins[i] = expIns[i]
		default:
			ins[i] = fcx.ic.newVar()
		}
	}
	var out Ty
	if x.Ret != nil {
		out = fcx.lowerTy(x.Ret)
	} else if expOut != nil {
		out = expOut
	} else {
		out = fcx.ic.newVar()
	}
	cty := Closure(fcx.def, int(x.ID), fcx.inst.Args)
	ci := &ClosureInfo{Ty: cty, Expr: x, Inputs: ins, Output: out, Results: fcx.r}
	fcx.t.Closures[cty] = ci
	fcx.r.Closures[x.ID] = ci
	fcx.closureParent[x.ID] = fcx.curClosure()
	fcx.closureExprs = append(fcx.closureExprs, x)
	fcx.closures = append(fcx.closures, &closureCtx{expr: x, retTy: out})
	fcx.pushScope()
	for i, p := range x.Params {
		if ip, ok := p.Pat.(*ast.IdentPat); ok && ip.Sub == nil && !ip.ByRef {
			if _, _, isPath := fcx.identPatAsPath(ip); !isPath {
				l := fcx.newLocal(ip.Name.Name, ip.Name.Hyg, ins[i], ip.Mut, ip.Name.Span, ip.ID)
				fcx.r.Locals[ip.ID] = l
				fcx.record(ip.ID, ins[i])
				ci.Params = append(ci.Params, l)
				continue
			}
		}
		fcx.localCount++
		l := &Local{ID: fcx.localCount, Name: "__arg", Ty: ins[i], Span: p.Pat.GetSpan(), Owner: x.ID}
		ci.Params = append(ci.Params, l)
		fcx.checkPat(p.Pat, ins[i], bmByValue, true)
	}
	fcx.checkExprCoerce(x.Body, out)
	fcx.popScope()
	fcx.closures = fcx.closures[:len(fcx.closures)-1]
	return cty
}

// noteLocalUse is a hook for local uses (captures are computed in writeback).
func (fcx *FnCtxt) noteLocalUse(l *Local, e *ast.PathExpr) {}

var lexerSpanZero lexer.Span
