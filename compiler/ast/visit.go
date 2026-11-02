package ast

// Visitor is called for every node during Walk. Nodes are *Item, *Block,
// Stmt, Expr, Pat, Type, *Arm, *Path. Returning false skips the children.
type Visitor func(n any) bool

// Walk traverses an AST node in source order.
func Walk(n any, v Visitor) {
	switch x := n.(type) {
	case nil:
		return
	case *Crate:
		for _, it := range x.Items {
			Walk(it, v)
		}
	case *Item:
		if x == nil || !v(x) {
			return
		}
		walkItemKind(x.Kind, v)
	case *Block:
		if x == nil || !v(x) {
			return
		}
		for _, s := range x.Stmts {
			Walk(s, v)
		}
	case Stmt:
		walkStmt(x, v)
	case Expr:
		walkExpr(x, v)
	case Pat:
		walkPat(x, v)
	case Type:
		walkType(x, v)
	case *Arm:
		if !v(x) {
			return
		}
		Walk(x.Pat, v)
		if x.Guard != nil {
			Walk(x.Guard, v)
		}
		Walk(x.Body, v)
	case *Path:
		walkPath(x, v)
	case *Bound:
		walkBound(x, v)
	case *WherePredicate:
		if x.Ty != nil {
			Walk(x.Ty, v)
		}
		for _, b := range x.Bounds {
			walkBound(b, v)
		}
	}
}

func walkGenerics(g *Generics, v Visitor) {
	if g == nil {
		return
	}
	for _, p := range g.Params {
		for _, b := range p.Bounds {
			walkBound(b, v)
		}
		if p.Default != nil {
			Walk(p.Default, v)
		}
		if p.ConstTy != nil {
			Walk(p.ConstTy, v)
		}
	}
	for _, w := range g.Where {
		if w.Ty != nil {
			Walk(w.Ty, v)
		}
		for _, b := range w.Bounds {
			walkBound(b, v)
		}
	}
}

func walkBound(b *Bound, v Visitor) {
	if b != nil && b.Path != nil {
		walkPath(b.Path, v)
	}
}

func walkPath(p *Path, v Visitor) {
	if p == nil || !v(p) {
		return
	}
	for _, s := range p.Segments {
		walkGenericArgs(s.Args, v)
	}
}

func walkGenericArgs(a *GenericArgs, v Visitor) {
	if a == nil {
		return
	}
	for _, ga := range a.Args {
		if ga.Type != nil {
			Walk(ga.Type, v)
		}
		if ga.Const != nil {
			Walk(ga.Const, v)
		}
	}
	for _, b := range a.Bindings {
		if b.Ty != nil {
			Walk(b.Ty, v)
		}
		for _, bd := range b.Bounds {
			walkBound(bd, v)
		}
	}
	for _, t := range a.Inputs {
		Walk(t, v)
	}
	if a.Output != nil {
		Walk(a.Output, v)
	}
}

func walkItemKind(k ItemKind, v Visitor) {
	switch it := k.(type) {
	case *FnItem:
		walkGenerics(it.Generics, v)
		if it.Decl.SelfParam != nil && it.Decl.SelfParam.Ty != nil {
			Walk(it.Decl.SelfParam.Ty, v)
		}
		for _, p := range it.Decl.Params {
			Walk(p.Pat, v)
			Walk(p.Ty, v)
		}
		if it.Decl.Output != nil {
			Walk(it.Decl.Output, v)
		}
		if it.Body != nil {
			Walk(it.Body, v)
		}
	case *StructItem:
		walkGenerics(it.Generics, v)
		for _, f := range it.Fields {
			Walk(f.Ty, v)
		}
	case *UnionItem:
		walkGenerics(it.Generics, v)
		for _, f := range it.Fields {
			Walk(f.Ty, v)
		}
	case *EnumItem:
		walkGenerics(it.Generics, v)
		for _, va := range it.Variants {
			for _, f := range va.Fields {
				Walk(f.Ty, v)
			}
			if va.Disc != nil {
				Walk(va.Disc, v)
			}
		}
	case *TraitItem:
		walkGenerics(it.Generics, v)
		for _, b := range it.Supertraits {
			walkBound(b, v)
		}
		for _, sub := range it.Items {
			Walk(sub, v)
		}
	case *ImplItem:
		walkGenerics(it.Generics, v)
		if it.Trait != nil {
			walkPath(it.Trait, v)
		}
		Walk(it.SelfTy, v)
		for _, sub := range it.Items {
			Walk(sub, v)
		}
	case *TypeAliasItem:
		walkGenerics(it.Generics, v)
		if it.Ty != nil {
			Walk(it.Ty, v)
		}
	case *ConstItem:
		if it.Ty != nil {
			Walk(it.Ty, v)
		}
		if it.Expr != nil {
			Walk(it.Expr, v)
		}
	case *StaticItem:
		Walk(it.Ty, v)
		if it.Expr != nil {
			Walk(it.Expr, v)
		}
	case *ModItem:
		for _, sub := range it.Items {
			Walk(sub, v)
		}
	case *ExternBlockItem:
		for _, sub := range it.Items {
			Walk(sub, v)
		}
	}
}

func walkStmt(s Stmt, v Visitor) {
	if !v(s) {
		return
	}
	switch x := s.(type) {
	case *LetStmt:
		Walk(x.Pat, v)
		if x.Ty != nil {
			Walk(x.Ty, v)
		}
		if x.Init != nil {
			Walk(x.Init, v)
		}
		if x.Else != nil {
			Walk(x.Else, v)
		}
	case *ExprStmt:
		Walk(x.Expr, v)
	case *ItemStmt:
		Walk(x.Item, v)
	}
}

func walkExpr(e Expr, v Visitor) {
	if e == nil || !v(e) {
		return
	}
	switch x := e.(type) {
	case *PathExpr:
		if x.QSelf != nil {
			Walk(x.QSelf.Ty, v)
		}
		walkPath(x.Path, v)
	case *UnaryExpr:
		Walk(x.X, v)
	case *BinaryExpr:
		Walk(x.L, v)
		Walk(x.R, v)
	case *AssignExpr:
		Walk(x.L, v)
		Walk(x.R, v)
	case *AssignOpExpr:
		Walk(x.L, v)
		Walk(x.R, v)
	case *CallExpr:
		Walk(x.Fn, v)
		for _, a := range x.Args {
			Walk(a, v)
		}
	case *MethodCallExpr:
		Walk(x.Receiver, v)
		walkGenericArgs(x.Generics, v)
		for _, a := range x.Args {
			Walk(a, v)
		}
	case *FieldExpr:
		Walk(x.X, v)
	case *IndexExpr:
		Walk(x.X, v)
		Walk(x.Index, v)
	case *RefExpr:
		Walk(x.X, v)
	case *CastExpr:
		Walk(x.X, v)
		Walk(x.Ty, v)
	case *AsmExpr:
		for _, op := range x.Operands {
			if op.In != nil {
				Walk(op.In, v)
			}
			if op.Out != nil && op.Out != op.In {
				Walk(op.Out, v)
			}
		}
	case *BlockExpr:
		Walk(x.Block, v)
	case *IfExpr:
		Walk(x.Cond, v)
		Walk(x.Then, v)
		if x.Else != nil {
			Walk(x.Else, v)
		}
	case *LetExpr:
		Walk(x.Pat, v)
		Walk(x.Expr, v)
	case *WhileExpr:
		Walk(x.Cond, v)
		Walk(x.Body, v)
	case *LoopExpr:
		Walk(x.Body, v)
	case *ForExpr:
		Walk(x.Pat, v)
		Walk(x.Iter, v)
		Walk(x.Body, v)
	case *MatchExpr:
		Walk(x.Scrutinee, v)
		for _, a := range x.Arms {
			Walk(a, v)
		}
	case *ClosureExpr:
		for _, p := range x.Params {
			Walk(p.Pat, v)
			if p.Ty != nil {
				Walk(p.Ty, v)
			}
		}
		if x.Ret != nil {
			Walk(x.Ret, v)
		}
		Walk(x.Body, v)
	case *BreakExpr:
		if x.Value != nil {
			Walk(x.Value, v)
		}
	case *ReturnExpr:
		if x.Value != nil {
			Walk(x.Value, v)
		}
	case *TupleExpr:
		for _, a := range x.Elems {
			Walk(a, v)
		}
	case *ArrayExpr:
		for _, a := range x.Elems {
			Walk(a, v)
		}
	case *RepeatExpr:
		Walk(x.Elem, v)
		Walk(x.Count, v)
	case *StructExpr:
		if x.QSelf != nil {
			Walk(x.QSelf.Ty, v)
		}
		walkPath(x.Path, v)
		for _, f := range x.Fields {
			Walk(f.Expr, v)
		}
		if x.Base != nil {
			Walk(x.Base, v)
		}
	case *RangeExpr:
		if x.Start != nil {
			Walk(x.Start, v)
		}
		if x.End != nil {
			Walk(x.End, v)
		}
	case *TryExpr:
		Walk(x.X, v)
	case *ParenExpr:
		Walk(x.X, v)
	case *AwaitExpr:
		Walk(x.X, v)
	case *FormatArgsExpr:
		for _, a := range x.Args {
			Walk(a, v)
		}
	}
}

func walkPat(p Pat, v Visitor) {
	if p == nil || !v(p) {
		return
	}
	switch x := p.(type) {
	case *IdentPat:
		if x.Sub != nil {
			Walk(x.Sub, v)
		}
	case *LitPat:
		Walk(x.Expr, v)
	case *RangePat:
		if x.Lo != nil {
			Walk(x.Lo, v)
		}
		if x.Hi != nil {
			Walk(x.Hi, v)
		}
	case *TuplePat:
		for _, e := range x.Elems {
			Walk(e, v)
		}
	case *TupleStructPat:
		walkPath(x.Path, v)
		for _, e := range x.Elems {
			Walk(e, v)
		}
	case *StructPat:
		walkPath(x.Path, v)
		for _, f := range x.Fields {
			Walk(f.Pat, v)
		}
	case *PathPat:
		walkPath(x.Path, v)
	case *RefPat:
		Walk(x.Pat, v)
	case *BoxPat:
		Walk(x.Pat, v)
	case *SlicePat:
		for _, e := range x.Elems {
			Walk(e, v)
		}
	case *OrPat:
		for _, e := range x.Alts {
			Walk(e, v)
		}
	case *ParenPat:
		Walk(x.Pat, v)
	}
}

func walkType(t Type, v Visitor) {
	if t == nil || !v(t) {
		return
	}
	switch x := t.(type) {
	case *PathType:
		if x.QSelf != nil {
			Walk(x.QSelf.Ty, v)
		}
		walkPath(x.Path, v)
	case *RefType:
		Walk(x.Elem, v)
	case *PtrType:
		Walk(x.Elem, v)
	case *SliceType:
		Walk(x.Elem, v)
	case *ArrayType:
		Walk(x.Elem, v)
		Walk(x.Len, v)
	case *TupleType:
		for _, e := range x.Elems {
			Walk(e, v)
		}
	case *FnPtrType:
		for _, e := range x.Inputs {
			Walk(e, v)
		}
		if x.Output != nil {
			Walk(x.Output, v)
		}
	case *ImplTraitType:
		for _, b := range x.Bounds {
			walkBound(b, v)
		}
	case *DynTraitType:
		for _, b := range x.Bounds {
			walkBound(b, v)
		}
	case *ParenType:
		Walk(x.Elem, v)
	}
}
