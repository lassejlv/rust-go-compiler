package expand

// Desugaring of async/await (after macro expansion):
//
//	async fn f(a: A) -> T { body }
//	  => fn f(a: A) -> impl Future<Output = T> { __async_block(move || -> T { body }) }
//	async move? { body }   => __async_block(move? || body)
//	async move? |x| body   => move? |x| __async_block(move || body)
//	e.await                => match IntoFuture::into_future(e) { mut f => loop {
//	                            match Future::poll(Pin::new_unchecked(&mut f), __current_cx()) {
//	                              Ready(v) => break v, Pending => {} }
//	                            __yield_now(); } }
//
// `__async_block` runs its closure on a coroutine (see std::future).

import (
	"rustgo/compiler/ast"
	"rustgo/compiler/lexer"
	"rustgo/compiler/parser"
)

const awaitTemplate = `match ::std::future::IntoFuture::into_future(__rg_await_operand) { mut __awaitee => loop { match ::std::future::Future::poll(unsafe { ::std::pin::Pin::new_unchecked(&mut __awaitee) }, ::std::future::__current_cx()) { ::std::task::Poll::Ready(__awv) => break __awv, ::std::task::Poll::Pending => {} } ::std::future::__yield_now(); } }`

// templExpr parses an expression template and substitutes single-segment
// paths named in subs.
func templExpr(src string, subs map[string]ast.Expr, sp lexer.Span) (ast.Expr, error) {
	tts, err := tmpl(sp, src)
	if err != nil {
		return nil, err
	}
	var e ast.Expr
	err = safeParse(func() error {
		var err error
		e, err = parser.NewFromTokenTrees(tts).ParseExpr()
		return err
	})
	if err != nil {
		return nil, err
	}
	if len(subs) == 0 {
		return e, nil
	}
	return ast.RewriteExpr(e, func(x ast.Expr) ast.Expr {
		if pe, ok := x.(*ast.PathExpr); ok && pe.QSelf == nil && len(pe.Path.Segments) == 1 && !pe.Path.Global {
			if r, ok := subs[pe.Path.Segments[0].Ident.Name]; ok {
				return r
			}
		}
		return x
	}), nil
}

func asyncBlockCall(body ast.Expr, move bool, ret ast.Type, sp lexer.Span) (ast.Expr, error) {
	fn, err := templExpr("::std::future::__async_block", nil, sp)
	if err != nil {
		return nil, err
	}
	clo := &ast.ClosureExpr{ExprBase: ast.ExprBase{Span: sp, ID: ast.NewID()}, Move: move, Body: body, Ret: ret}
	return &ast.CallExpr{ExprBase: ast.ExprBase{Span: sp, ID: ast.NewID()}, Fn: fn, Args: []ast.Expr{clo}}, nil
}

func futureOutputType(ret ast.Type, sp lexer.Span) ast.Type {
	seg := func(n string) *ast.PathSegment { return &ast.PathSegment{Ident: ast.Ident{Name: n, Span: sp}} }
	fut := seg("Future")
	fut.Args = &ast.GenericArgs{Bindings: []*ast.AssocBinding{{Name: ast.Ident{Name: "Output", Span: sp}, Ty: ret}}, Span: sp}
	p := &ast.Path{Global: true, Segments: []*ast.PathSegment{seg("std"), seg("future"), fut}, Span: sp, ID: ast.NewID()}
	return &ast.ImplTraitType{TypeBase: ast.TypeBase{Span: sp, ID: ast.NewID()}, Bounds: []*ast.Bound{{Path: p, Span: sp}}}
}

// desugarAsync rewrites async constructs in a crate.
func desugarAsync(c *ast.Crate) error {
	var firstErr error
	fail := func(err error) {
		if firstErr == nil {
			firstErr = err
		}
	}
	rewrite := func(x ast.Expr) ast.Expr {
		switch e := x.(type) {
		case *ast.AwaitExpr:
			r, err := templExpr(awaitTemplate, map[string]ast.Expr{"__rg_await_operand": e.X}, e.Span)
			if err != nil {
				fail(err)
				return x
			}
			return r
		case *ast.BlockExpr:
			if !e.Async {
				return x
			}
			e.Async = false
			body := &ast.BlockExpr{ExprBase: ast.ExprBase{Span: e.Span, ID: ast.NewID()}, Block: e.Block}
			r, err := asyncBlockCall(body, e.Move, nil, e.Span)
			if err != nil {
				fail(err)
				return x
			}
			return r
		case *ast.ClosureExpr:
			if !e.Async {
				return x
			}
			e.Async = false
			inner, err := asyncBlockCall(e.Body, true, e.Ret, e.Span)
			if err != nil {
				fail(err)
				return x
			}
			e.Body = inner
			e.Ret = nil
			return e
		}
		return x
	}
	var items func([]*ast.Item)
	items = func(its []*ast.Item) {
		for _, it := range its {
			switch k := it.Kind.(type) {
			case *ast.FnItem:
				if k.Body != nil {
					ast.RewriteExprs(k.Body, rewrite)
				}
				if k.Async && k.Body != nil {
					k.Async = false
					ret := k.Decl.Output
					if ret == nil {
						ret = &ast.TupleType{TypeBase: ast.TypeBase{Span: it.Span, ID: ast.NewID()}}
					}
					body := &ast.BlockExpr{ExprBase: ast.ExprBase{Span: k.Body.Span, ID: ast.NewID()}, Block: k.Body}
					call, err := asyncBlockCall(body, true, ret, it.Span)
					if err != nil {
						fail(err)
						continue
					}
					k.Body = &ast.Block{Stmts: []ast.Stmt{&ast.ExprStmt{Expr: call}}, Span: k.Body.Span, ID: ast.NewID()}
					k.Decl.Output = futureOutputType(ret, it.Span)
				}
				// nested items inside the body
				ast.Walk(k.Body, func(n any) bool {
					if is, ok := n.(*ast.ItemStmt); ok {
						items([]*ast.Item{is.Item})
						return false
					}
					return true
				})
			case *ast.ImplItem:
				items(k.Items)
			case *ast.TraitItem:
				items(k.Items)
			case *ast.ModItem:
				items(k.Items)
			case *ast.ConstItem, *ast.StaticItem:
				ast.RewriteExprs(it, rewrite)
			}
		}
	}
	items(c.Items)
	return firstErr
}
