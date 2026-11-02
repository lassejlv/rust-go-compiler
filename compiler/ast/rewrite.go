package ast

// RewriteExprs rewrites every expression reachable from n bottom-up: the
// children of an expression are rewritten first, then f is applied to the
// expression itself and its result replaces it. Items, blocks and
// statements are traversed (but not types or patterns).
func RewriteExprs(n any, f func(Expr) Expr) {
	rw := &rewriter{f: f}
	rw.any(n)
}

type rewriter struct {
	f func(Expr) Expr
}

func (rw *rewriter) any(n any) {
	switch x := n.(type) {
	case *Crate:
		for _, it := range x.Items {
			rw.item(it)
		}
	case *Item:
		rw.item(x)
	case *Block:
		rw.block(x)
	case Expr:
		rw.expr(x)
	}
}

func (rw *rewriter) items(items []*Item) {
	for _, it := range items {
		rw.item(it)
	}
}

func (rw *rewriter) item(it *Item) {
	if it == nil {
		return
	}
	switch k := it.Kind.(type) {
	case *FnItem:
		if k.Body != nil {
			rw.block(k.Body)
		}
	case *TraitItem:
		rw.items(k.Items)
	case *ImplItem:
		rw.items(k.Items)
	case *ConstItem:
		if k.Expr != nil {
			k.Expr = rw.expr(k.Expr)
		}
	case *StaticItem:
		if k.Expr != nil {
			k.Expr = rw.expr(k.Expr)
		}
	case *ModItem:
		rw.items(k.Items)
	case *EnumItem:
		for _, v := range k.Variants {
			if v.Disc != nil {
				v.Disc = rw.expr(v.Disc)
			}
		}
	}
}

func (rw *rewriter) block(b *Block) {
	if b == nil {
		return
	}
	for _, s := range b.Stmts {
		switch x := s.(type) {
		case *LetStmt:
			if x.Init != nil {
				x.Init = rw.expr(x.Init)
			}
			if x.Else != nil {
				rw.block(x.Else)
			}
		case *ExprStmt:
			x.Expr = rw.expr(x.Expr)
		case *ItemStmt:
			rw.item(x.Item)
		}
	}
}

func (rw *rewriter) opt(e Expr) Expr {
	if e == nil {
		return nil
	}
	return rw.expr(e)
}

func (rw *rewriter) list(es []Expr) {
	for i, e := range es {
		es[i] = rw.expr(e)
	}
}

func (rw *rewriter) expr(e Expr) Expr {
	switch x := e.(type) {
	case nil:
		return nil
	case *UnaryExpr:
		x.X = rw.expr(x.X)
	case *BinaryExpr:
		x.L = rw.expr(x.L)
		x.R = rw.expr(x.R)
	case *AssignExpr:
		x.L = rw.expr(x.L)
		x.R = rw.expr(x.R)
	case *AssignOpExpr:
		x.L = rw.expr(x.L)
		x.R = rw.expr(x.R)
	case *CallExpr:
		x.Fn = rw.expr(x.Fn)
		rw.list(x.Args)
	case *MethodCallExpr:
		x.Receiver = rw.expr(x.Receiver)
		rw.list(x.Args)
	case *FieldExpr:
		x.X = rw.expr(x.X)
	case *IndexExpr:
		x.X = rw.expr(x.X)
		x.Index = rw.expr(x.Index)
	case *RefExpr:
		x.X = rw.expr(x.X)
	case *CastExpr:
		x.X = rw.expr(x.X)
	case *BlockExpr:
		rw.block(x.Block)
	case *IfExpr:
		x.Cond = rw.expr(x.Cond)
		rw.block(x.Then)
		x.Else = rw.opt(x.Else)
	case *LetExpr:
		x.Expr = rw.expr(x.Expr)
	case *WhileExpr:
		x.Cond = rw.expr(x.Cond)
		rw.block(x.Body)
	case *LoopExpr:
		rw.block(x.Body)
	case *ForExpr:
		x.Iter = rw.expr(x.Iter)
		rw.block(x.Body)
	case *MatchExpr:
		x.Scrutinee = rw.expr(x.Scrutinee)
		for _, a := range x.Arms {
			a.Guard = rw.opt(a.Guard)
			a.Body = rw.expr(a.Body)
		}
	case *ClosureExpr:
		x.Body = rw.expr(x.Body)
	case *BreakExpr:
		x.Value = rw.opt(x.Value)
	case *ReturnExpr:
		x.Value = rw.opt(x.Value)
	case *TupleExpr:
		rw.list(x.Elems)
	case *ArrayExpr:
		rw.list(x.Elems)
	case *RepeatExpr:
		x.Elem = rw.expr(x.Elem)
		x.Count = rw.expr(x.Count)
	case *StructExpr:
		for _, f := range x.Fields {
			f.Expr = rw.expr(f.Expr)
		}
		x.Base = rw.opt(x.Base)
	case *RangeExpr:
		x.Start = rw.opt(x.Start)
		x.End = rw.opt(x.End)
	case *TryExpr:
		x.X = rw.expr(x.X)
	case *ParenExpr:
		x.X = rw.expr(x.X)
	case *AwaitExpr:
		x.X = rw.expr(x.X)
	case *FormatArgsExpr:
		rw.list(x.Args)
	}
	return rw.f(e)
}

// RewriteExpr is RewriteExprs for a root expression, returning the
// (possibly replaced) root.
func RewriteExpr(e Expr, f func(Expr) Expr) Expr {
	rw := &rewriter{f: f}
	return rw.expr(e)
}
