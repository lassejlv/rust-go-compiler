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
	"fmt"
	"strings"
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

// asyncBlockCall builds `__async_block(move? || body)`. The call is
// #[track_caller]: its span `loc` is where the future reports being polled
// after completion or after panicking (the `async` keyword of a block, the
// body of an async fn or closure, like rustc).
func asyncBlockCall(body ast.Expr, move bool, ret ast.Type, sp, loc lexer.Span) (ast.Expr, error) {
	fn, err := templExpr("::std::future::__async_block", nil, sp)
	if err != nil {
		return nil, err
	}
	clo := &ast.ClosureExpr{ExprBase: ast.ExprBase{Span: sp, ID: ast.NewID()}, Move: move, Body: body, Ret: ret}
	return &ast.CallExpr{ExprBase: ast.ExprBase{Span: loc, ID: ast.NewID()}, Fn: fn, Args: []ast.Expr{clo}}, nil
}

// implTraitToInfer returns t with every `impl Trait` replaced by `_`: the
// return type annotation of the closure an async fn's body becomes (its
// declared type keeps the opaque types, which the body's type defines).
func implTraitToInfer(t ast.Type) ast.Type {
	base := func(b ast.TypeBase) ast.TypeBase { return ast.TypeBase{Span: b.Span, ID: ast.NewID()} }
	list := func(ts []ast.Type) ([]ast.Type, bool) {
		var out []ast.Type
		for i, e := range ts {
			if n := implTraitToInfer(e); n != e {
				if out == nil {
					out = append([]ast.Type(nil), ts...)
				}
				out[i] = n
			}
		}
		return out, out != nil
	}
	switch x := t.(type) {
	case *ast.ImplTraitType:
		return &ast.InferType{TypeBase: base(x.TypeBase)}
	case *ast.ParenType:
		if e := implTraitToInfer(x.Elem); e != x.Elem {
			return &ast.ParenType{TypeBase: base(x.TypeBase), Elem: e}
		}
	case *ast.RefType:
		if e := implTraitToInfer(x.Elem); e != x.Elem {
			return &ast.RefType{TypeBase: base(x.TypeBase), Lifetime: x.Lifetime, Mut: x.Mut, Elem: e}
		}
	case *ast.PtrType:
		if e := implTraitToInfer(x.Elem); e != x.Elem {
			return &ast.PtrType{TypeBase: base(x.TypeBase), Mut: x.Mut, Elem: e}
		}
	case *ast.SliceType:
		if e := implTraitToInfer(x.Elem); e != x.Elem {
			return &ast.SliceType{TypeBase: base(x.TypeBase), Elem: e}
		}
	case *ast.ArrayType:
		if e := implTraitToInfer(x.Elem); e != x.Elem {
			return &ast.ArrayType{TypeBase: base(x.TypeBase), Elem: e, Len: x.Len}
		}
	case *ast.TupleType:
		if es, ok := list(x.Elems); ok {
			return &ast.TupleType{TypeBase: base(x.TypeBase), Elems: es}
		}
	case *ast.PathType:
		if x.QSelf != nil || x.Path == nil {
			return t
		}
		var segs []*ast.PathSegment
		for i, sg := range x.Path.Segments {
			if sg.Args == nil || sg.Args.Parenthesized {
				continue
			}
			var args []ast.GenericArg
			for j, a := range sg.Args.Args {
				if a.Type == nil {
					continue
				}
				if n := implTraitToInfer(a.Type); n != a.Type {
					if args == nil {
						args = append([]ast.GenericArg(nil), sg.Args.Args...)
					}
					args[j].Type = n
				}
			}
			if args == nil {
				continue
			}
			if segs == nil {
				segs = append([]*ast.PathSegment(nil), x.Path.Segments...)
			}
			ga := *sg.Args
			ga.Args = args
			segs[i] = &ast.PathSegment{Ident: sg.Ident, Args: &ga}
		}
		if segs != nil {
			p := &ast.Path{Global: x.Path.Global, Segments: segs, Span: x.Path.Span, ID: ast.NewID()}
			return &ast.PathType{TypeBase: base(x.TypeBase), Path: p}
		}
	}
	return t
}

func futureOutputType(ret ast.Type, sp lexer.Span) ast.Type {
	seg := func(n string) *ast.PathSegment { return &ast.PathSegment{Ident: ast.Ident{Name: n, Span: sp}} }
	fut := seg("Future")
	fut.Args = &ast.GenericArgs{Bindings: []*ast.AssocBinding{{Name: ast.Ident{Name: "Output", Span: sp}, Ty: ret}}, Span: sp}
	p := &ast.Path{Global: true, Segments: []*ast.PathSegment{seg("std"), seg("future"), fut}, Span: sp, ID: ast.NewID()}
	return &ast.ImplTraitType{TypeBase: ast.TypeBase{Span: sp, ID: ast.NewID()}, Bounds: []*ast.Bound{{Path: p, Span: sp}}}
}

// moveAsyncParams makes the future of an async fn own every parameter, like
// rustc: the parameters are re-bound by statements at the start of the
// future's body (`let x = x;`, or `let mut __rg_argN = __rg_argN; let <pat>
// = __rg_argN;` for other patterns), so that unused parameters, and those
// only read through Copy fields, live (and are dropped) with the future
// rather than when it is created. A by-value `self` is captured whole.
func moveAsyncParams(d *ast.FnDecl) []ast.Stmt {
	var lets []ast.Stmt
	if sp := d.SelfParam; sp != nil && (sp.Kind == ast.SelfValue || sp.Kind == ast.SelfTyped) {
		// `let _ = &self;` would not capture it; a dead borrow does
		self := ast.Ident{Name: "self", Span: sp.Span}
		keep := ast.Ident{Name: "__rg_self", Span: sp.Span}
		ref := &ast.RefExpr{ExprBase: ast.ExprBase{Span: sp.Span, ID: ast.NewID()}, X: singlePath(self)}
		lets = append(lets, &ast.LetStmt{Pat: identPat(keep, false), Init: ref, Span: sp.Span, ID: ast.NewID()})
	}
	for i, p := range d.Params {
		lets = append(lets, moveParam(&p.Pat, i, p.Span)...)
	}
	return lets
}

// moveClosureParams is moveAsyncParams for an async closure.
func moveClosureParams(c *ast.ClosureExpr) []ast.Stmt {
	var lets []ast.Stmt
	for i, p := range c.Params {
		lets = append(lets, moveParam(&p.Pat, i, p.Pat.GetSpan())...)
	}
	return lets
}

// moveParam replaces parameter pattern *pp by a plain binding and returns
// the statements re-binding the original pattern.
func moveParam(pp *ast.Pat, i int, sp lexer.Span) []ast.Stmt {
	if ip, ok := (*pp).(*ast.IdentPat); ok && !ip.ByRef && ip.Sub == nil {
		// `let x = x;` (the parameter itself becomes immutable)
		*pp = identPat(ip.Name, false)
		return []ast.Stmt{&ast.LetStmt{Pat: identPat(ip.Name, ip.Mut), Init: singlePath(ip.Name), Span: sp, ID: ast.NewID()}}
	}
	arg := ast.Ident{Name: fmt.Sprintf("__rg_arg%d", i), Span: (*pp).GetSpan()}
	pat := *pp
	*pp = identPat(arg, false)
	return []ast.Stmt{
		&ast.LetStmt{Pat: identPat(arg, true), Init: singlePath(arg), Span: sp, ID: ast.NewID()},
		&ast.LetStmt{Pat: pat, Init: singlePath(arg), Span: sp, ID: ast.NewID()},
	}
}

func singlePath(id ast.Ident) ast.Expr {
	return &ast.PathExpr{ExprBase: ast.ExprBase{Span: id.Span, ID: ast.NewID()},
		Path: &ast.Path{Segments: []*ast.PathSegment{{Ident: id}}, Span: id.Span, ID: ast.NewID()}}
}

func identPat(id ast.Ident, mut bool) ast.Pat {
	return &ast.IdentPat{PatBase: ast.PatBase{Span: id.Span, ID: ast.NewID()}, Name: id, Mut: mut}
}

// prependStmts returns a block expression running stmts, then body.
func prependStmts(stmts []ast.Stmt, body ast.Expr, sp lexer.Span) ast.Expr {
	if len(stmts) == 0 {
		return body
	}
	stmts = append(stmts, &ast.ExprStmt{Expr: body})
	return &ast.BlockExpr{ExprBase: ast.ExprBase{Span: sp, ID: ast.NewID()}, Block: &ast.Block{Stmts: stmts, Span: sp, ID: ast.NewID()}}
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
		case *ast.LitExpr:
			if e.Kind != ast.LitCStr {
				return x
			}
			// c"..." => CStr::from_bytes_with_nul_unchecked(b"...\0")
			if strings.ContainsRune(e.Value, 0) {
				fail(errAt(e.Span, "null characters in C string literals are not supported"))
				return x
			}
			r, err := templExpr("unsafe { ::std::ffi::CStr::from_bytes_with_nul_unchecked(b"+quoteRust(e.Value+"\x00", '"')+") }", nil, e.Span)
			if err != nil {
				fail(err)
				return x
			}
			return r
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
			r, err := asyncBlockCall(body, e.Move, nil, e.Span, e.Span)
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
			body := prependStmts(moveClosureParams(e), e.Body, e.Body.GetSpan())
			inner, err := asyncBlockCall(body, true, e.Ret, e.Span, e.Body.GetSpan())
			if err != nil {
				fail(err)
				return x
			}
			if !e.Move {
				inner.(*ast.CallExpr).Args[0].(*ast.ClosureExpr).MoveParamsOf = e
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
					if err := checkAwaits(k.Body, k.Async); err != nil {
						fail(err)
					}
					ast.RewriteExprs(k.Body, rewrite)
				}
				if k.Async && k.Body == nil {
					// trait method declaration: return type becomes impl Future
					k.Async = false
					ret := k.Decl.Output
					if ret == nil {
						ret = &ast.TupleType{TypeBase: ast.TypeBase{Span: it.Span, ID: ast.NewID()}}
					}
					k.Decl.Output = futureOutputType(ret, it.Span)
				}
				if k.Async && k.Body != nil {
					k.Async = false
					ret := k.Decl.Output
					if ret == nil {
						ret = &ast.TupleType{TypeBase: ast.TypeBase{Span: it.Span, ID: ast.NewID()}}
					}
					var body ast.Expr = &ast.BlockExpr{ExprBase: ast.ExprBase{Span: k.Body.Span, ID: ast.NewID()}, Block: k.Body}
					body = prependStmts(moveAsyncParams(k.Decl), body, k.Body.Span)
					call, err := asyncBlockCall(body, true, implTraitToInfer(ret), it.Span, k.Body.Span)
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

// checkAwaits reports an `.await` outside async fns, blocks and closures
// (E0728).
func checkAwaits(n any, inAsync bool) error {
	var err error
	ast.Walk(n, func(x any) bool {
		if err != nil {
			return false
		}
		switch e := x.(type) {
		case *ast.ItemStmt:
			return false // checked with its own item
		case *ast.BlockExpr:
			if e.Async && !inAsync {
				err = checkAwaits(e.Block, true)
				return false
			}
		case *ast.ClosureExpr:
			if e.Async != inAsync {
				err = checkAwaits(e.Body, e.Async)
				return false
			}
		case *ast.AwaitExpr:
			if !inAsync {
				err = fmt.Errorf("%s: error[E0728]: `await` is only allowed inside `async` functions and blocks", e.Span.Lo)
				return false
			}
		}
		return true
	})
	return err
}
