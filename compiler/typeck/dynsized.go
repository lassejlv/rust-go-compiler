package typeck

import (
	"rustgo/compiler/ast"
	. "rustgo/compiler/middle"
)

// checkPackedRef reports a reference to a field of a packed struct whose
// type needs a greater alignment than the packing gives (E0793).
func (t *Tcx) checkPackedRef(r *Results, e ast.Expr, sp ast.Span) {
	fe, ok := unparen(e).(*ast.FieldExpr)
	if !ok {
		return
	}
	bt := r.AdjustedType(fe.X)
	for bt != nil && (bt.Kind == TRef || bt.Kind == TPtr) {
		bt = bt.Elem
	}
	if bt == nil || bt.Kind != TAdt || bt.Def == nil || bt.Def.Adt == nil || bt.Def.Adt.Kind == AdtUnion {
		return
	}
	pack := bt.Def.ReprPacked()
	if pack == 0 {
		return
	}
	ft := r.NodeTypes[fe.ID]
	if ft == nil || ft.HasParam() || ft.HasInfer() || ft.HasError() || ft.HasProj() || ft.HasOpaque() {
		return
	}
	if _, align, ok := t.CLayout(ft); ok && align > pack {
		t.ErrorCode(sp, "E0793", "reference to field of packed struct is unaligned")
	}
}

// checkDynSizedMethod reports a call of a trait method that requires
// `Self: Sized` on a trait object (rustc: "the method cannot be invoked on
// a trait object").
func (t *Tcx) checkDynSizedMethod(c *Callee, sp ast.Span) {
	if c == nil || (c.Kind != CallFn && c.Kind != CallVirtual) || c.Def == nil || !c.Def.InTrait() || len(c.Args) == 0 || c.Args[0] == nil || c.Args[0].Kind != TDyn {
		return
	}
	if whereSelfSized(genericsAst(c.Def)) {
		t.Errorf(sp, "the `%s` method cannot be invoked on a trait object", c.Def.Name)
	}
}
