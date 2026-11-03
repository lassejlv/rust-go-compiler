package parser

import (
	"rustgo/compiler/ast"
	"rustgo/compiler/lexer"
)

type pathStyle int

const (
	styleExpr pathStyle = iota // generic args only with `::<`
	styleType                  // `Vec<T>`, `Fn(A) -> B`
	styleMod                   // no generic args (use paths, visibility)
)

// isPathSegStart reports whether t can start a path segment.
func isPathSegStart(t lexer.Token) bool {
	if t.Kind == lexer.Ident {
		if t.Raw || !lexer.Keywords[t.Text] {
			return true
		}
		switch t.Text {
		case "self", "super", "crate", "Self":
			return true
		}
		return false
	}
	return t.Is("$")
}

func (p *Parser) parsePathSegIdent() ast.Ident {
	t := p.cur()
	if t.Is("$") && p.peekN(1).Kind == lexer.Ident && p.peekN(1).Text == "crate" {
		p.next()
		c := p.next()
		return ast.Ident{Name: "$crate", Span: lexer.Span{Lo: t.Span.Lo, Hi: c.Span.Hi}}
	}
	if !isPathSegStart(t) || t.Is("$") {
		p.errExpected("identifier")
	}
	p.next()
	return ast.Ident{Name: t.Text, Span: t.Span, Hyg: t.Hyg}
}

// parsePath parses a path in the given style.
func (p *Parser) parsePath(style pathStyle) *ast.Path {
	lo := p.lo()
	path := &ast.Path{}
	if p.is("::") {
		p.next()
		path.Global = true
	}
	path.Segments = p.parsePathSegments(style)
	path.Span = p.span(lo)
	path.ID = ast.NewID()
	return path
}

func (p *Parser) parsePathSegments(style pathStyle) []*ast.PathSegment {
	var segs []*ast.PathSegment
	for {
		seg := &ast.PathSegment{Ident: p.parsePathSegIdent()}
		segs = append(segs, seg)
		switch style {
		case styleType:
			if p.isGenericArgsStart() {
				seg.Args = p.parseGenericArgs()
			} else if p.is("::") && (p.isPrefixN(1, "<")) {
				p.next()
				seg.Args = p.parseGenericArgs()
			} else if p.is("(") && p.split == 0 {
				seg.Args = p.parseParenArgs()
			}
		case styleExpr:
			if p.is("::") && p.isPrefixN(1, "<") {
				p.next()
				seg.Args = p.parseGenericArgs()
			}
		}
		if p.is("::") && isPathSegStart(p.peekN(1)) {
			p.next()
			continue
		}
		return segs
	}
}

func (p *Parser) isPrefixN(n int, s string) bool {
	t := p.peekN(n)
	return t.Kind == lexer.Punct && len(t.Text) >= len(s) && t.Text[:len(s)] == s
}

// isGenericArgsStart reports whether a type path segment is followed by
// generic arguments `<...>` (and not e.g. `<=`).
func (p *Parser) isGenericArgsStart() bool {
	if p.noTypeLt {
		return false
	}
	t := p.cur()
	return t.Kind == lexer.Punct && (t.Text == "<" || t.Text == "<<")
}

func (p *Parser) isGt() bool { return p.isPrefix(">") }

func (p *Parser) parseGenericArgs() *ast.GenericArgs {
	lo := p.lo()
	p.expectPrefix("<")
	saveNT := p.noTypeLt
	p.noTypeLt = false
	ga := &ast.GenericArgs{}
	for !p.isGt() {
		p.parseGenericArg(ga)
		if !p.eat(",") {
			break
		}
	}
	p.expectPrefix(">")
	p.noTypeLt = saveNT
	ga.Span = p.span(lo)
	return ga
}

func isLitTok(t lexer.Token) bool {
	switch t.Kind {
	case lexer.Int, lexer.Float, lexer.Str, lexer.ByteStr, lexer.CStr, lexer.Char, lexer.Byte:
		return true
	case lexer.Ident:
		return !t.Raw && (t.Text == "true" || t.Text == "false")
	}
	return false
}

func (p *Parser) parseGenericArg(ga *ast.GenericArgs) {
	t := p.cur()
	switch {
	case t.Kind == lexer.Lifetime:
		p.next()
		ga.Args = append(ga.Args, ast.GenericArg{Lifetime: t.Text})
		return
	case t.Is("{"):
		ga.Args = append(ga.Args, ast.GenericArg{Const: p.parseBlockExpr("")})
		return
	case isLitTok(t):
		ga.Args = append(ga.Args, ast.GenericArg{Const: p.parseLit()})
		return
	case t.Is("-") && isLitTok(p.peekN(1)):
		lo := p.lo()
		p.next()
		lit := p.parseLit()
		ga.Args = append(ga.Args, ast.GenericArg{Const: &ast.UnaryExpr{ExprBase: p.eb(lo), Op: ast.UnNeg, X: lit}})
		return
	case isPlainIdent(t) && p.isN(1, "="):
		name := p.parseIdentTok()
		p.next()
		b := &ast.AssocBinding{Name: name}
		if p.is("{") || isLitTok(p.cur()) {
			// associated const equality; represent the value as a type-less
			// binding is not supported, parse and drop into a const arg.
			ga.Args = append(ga.Args, ast.GenericArg{Const: p.parseConstArgExpr()})
			return
		}
		b.Ty = p.parseType()
		ga.Bindings = append(ga.Bindings, b)
		return
	case isPlainIdent(t) && p.isN(1, "<") && p.gatBindingAhead():
		// GAT binding `Name<Args> = Type` / `Name<Args>: Bounds`
		name := p.parseIdentTok()
		args := p.parseGenericArgs()
		b := &ast.AssocBinding{Name: name, Args: args}
		if p.eat("=") {
			b.Ty = p.parseType()
		} else {
			p.expect(":")
			b.Bounds = p.parseBounds(true)
		}
		ga.Bindings = append(ga.Bindings, b)
		return
	case isPlainIdent(t) && p.isN(1, ":") && !p.isN(1, "::"):
		name := p.parseIdentTok()
		p.next()
		b := &ast.AssocBinding{Name: name, Bounds: p.parseBounds(true)}
		ga.Bindings = append(ga.Bindings, b)
		return
	}
	ga.Args = append(ga.Args, ast.GenericArg{Type: p.parseType()})
}

func (p *Parser) parseConstArgExpr() ast.Expr {
	if p.is("{") {
		return p.parseBlockExpr("")
	}
	if p.is("-") {
		lo := p.lo()
		p.next()
		lit := p.parseLit()
		return &ast.UnaryExpr{ExprBase: p.eb(lo), Op: ast.UnNeg, X: lit}
	}
	if isLitTok(p.cur()) {
		return p.parseLit()
	}
	lo := p.lo()
	path := p.parsePath(styleExpr)
	return &ast.PathExpr{ExprBase: p.eb(lo), Path: path}
}

// parseParenArgs parses `(A, B) -> C` generic args of Fn-like traits.
func (p *Parser) parseParenArgs() *ast.GenericArgs {
	lo := p.lo()
	p.expect("(")
	ga := &ast.GenericArgs{Parenthesized: true}
	for !p.is(")") {
		ga.Inputs = append(ga.Inputs, p.parseType())
		if !p.eat(",") {
			break
		}
	}
	p.expect(")")
	if p.eat("->") {
		ga.Output = p.parseTypeNoPlus()
	}
	ga.Span = p.span(lo)
	return ga
}

// parseQPath parses `<T as Trait>::rest` / `<T>::rest`.
func (p *Parser) parseQPath(style pathStyle) (*ast.QSelf, *ast.Path) {
	lo := p.lo()
	p.expectPrefix("<")
	saveNT := p.noTypeLt
	p.noTypeLt = false
	ty := p.parseType()
	path := &ast.Path{}
	pos := 0
	if p.eat("as") {
		tp := p.parsePath(styleType)
		path.Global = tp.Global
		path.Segments = tp.Segments
		pos = len(tp.Segments)
	}
	p.expectPrefix(">")
	p.noTypeLt = saveNT
	p.expect("::")
	path.Segments = append(path.Segments, p.parsePathSegments(style)...)
	path.Span = p.span(lo)
	path.ID = ast.NewID()
	return &ast.QSelf{Ty: ty, Position: pos}, path
}

// ---------------------------------------------------------------------------
// Types

func (p *Parser) parseType() ast.Type       { return p.parseTypeX(true) }
func (p *Parser) parseTypeNoPlus() ast.Type { return p.parseTypeX(false) }

func (p *Parser) parseTypeX(plus bool) ast.Type {
	lo := p.lo()
	t := p.cur()
	switch {
	case t.Is("("):
		p.next()
		if p.eat(")") {
			return &ast.TupleType{TypeBase: p.tb(lo)}
		}
		first := p.parseType()
		if p.eat(")") {
			pt := &ast.ParenType{TypeBase: p.tb(lo), Elem: first}
			if plus && p.is("+") {
				// (Trait) + Send — a bare trait object.
				if b := typeToBound(first); b != nil {
					bounds := []*ast.Bound{b}
					for p.eat("+") {
						if !p.canBeginBound() {
							break
						}
						bounds = append(bounds, p.parseBound())
					}
					return &ast.DynTraitType{TypeBase: p.tb(lo), Bounds: bounds}
				}
			}
			return pt
		}
		elems := []ast.Type{first}
		for p.eat(",") {
			if p.is(")") {
				break
			}
			elems = append(elems, p.parseType())
		}
		p.expect(")")
		return &ast.TupleType{TypeBase: p.tb(lo), Elems: elems}
	case t.Is("!"):
		p.next()
		return &ast.NeverType{TypeBase: p.tb(lo)}
	case t.Is("_"):
		p.next()
		return &ast.InferType{TypeBase: p.tb(lo)}
	case t.Is("&") || t.Is("&&"):
		p.eatPrefix("&")
		rt := &ast.RefType{}
		if p.cur().Kind == lexer.Lifetime {
			rt.Lifetime = p.next().Text
		}
		rt.Mut = p.eat("mut")
		rt.Elem = p.parseTypeNoPlus()
		rt.TypeBase = p.tb(lo)
		return rt
	case t.Is("*"):
		p.next()
		pt := &ast.PtrType{}
		if p.eat("mut") {
			pt.Mut = true
		} else if !p.eat("const") {
			p.errExpected("`mut` or `const` in raw pointer type")
		}
		pt.Elem = p.parseTypeNoPlus()
		pt.TypeBase = p.tb(lo)
		return pt
	case t.Is("["):
		p.next()
		elem := p.parseType()
		if p.eat(";") {
			n := p.parseExpr()
			p.expect("]")
			return &ast.ArrayType{TypeBase: p.tb(lo), Elem: elem, Len: n}
		}
		p.expect("]")
		return &ast.SliceType{TypeBase: p.tb(lo), Elem: elem}
	case t.Is("fn") || t.Is("unsafe") || t.Is("extern"):
		return p.parseFnPtrType(lo)
	case t.Is("for"):
		lts := p.parseForLifetimes()
		if p.is("fn") || p.is("unsafe") || p.is("extern") {
			return p.parseFnPtrType(lo)
		}
		b := p.parseBound()
		b.ForLifetimes = append(lts, b.ForLifetimes...)
		bounds := []*ast.Bound{b}
		if plus {
			bounds = p.parseMoreBounds(bounds)
		}
		return &ast.DynTraitType{TypeBase: p.tb(lo), Bounds: bounds}
	case t.Is("impl"):
		p.next()
		return &ast.ImplTraitType{Bounds: p.parseBounds(plus), TypeBase: p.tb(lo)}
	case t.Is("dyn"):
		p.next()
		return &ast.DynTraitType{Bounds: p.parseBounds(plus), TypeBase: p.tb(lo)}
	case t.Is("?"):
		// `?Sized` used as a bare trait object (only valid in bounds); parse
		// as a dyn type for robustness.
		return &ast.DynTraitType{Bounds: p.parseBounds(plus), TypeBase: p.tb(lo)}
	case t.Is("<") || t.Is("<<"):
		qs, path := p.parseQPath(styleType)
		return &ast.PathType{TypeBase: p.tb(lo), QSelf: qs, Path: path}
	case t.Is("::") || isPathSegStart(t):
		path := p.parsePath(styleType)
		if p.is("!") && (p.isN(1, "(") || p.isN(1, "[") || p.isN(1, "{")) && !pathHasArgs(path) {
			p.next()
			g := p.parseDelimGroup()
			mac := &ast.MacCall{Path: path, Delim: g.Delim, Args: g.Inner, Span: p.span(lo)}
			return &ast.MacroType{TypeBase: p.tb(lo), Mac: mac}
		}
		if plus && p.is("+") {
			bounds := []*ast.Bound{{Path: path, Span: path.Span}}
			bounds = p.parseMoreBounds(bounds)
			return &ast.DynTraitType{TypeBase: p.tb(lo), Bounds: bounds}
		}
		return &ast.PathType{TypeBase: p.tb(lo), Path: path}
	}
	p.errExpected("type")
	return nil
}

func typeToBound(t ast.Type) *ast.Bound {
	if pt, ok := t.(*ast.PathType); ok && pt.QSelf == nil {
		return &ast.Bound{Path: pt.Path, Span: pt.Span}
	}
	return nil
}

func pathHasArgs(p *ast.Path) bool {
	for _, s := range p.Segments {
		if s.Args != nil {
			return true
		}
	}
	return false
}

func (p *Parser) parseFnPtrType(lo lexer.Pos) ast.Type {
	ft := &ast.FnPtrType{}
	if p.eat("unsafe") {
		ft.Unsafe = true
	}
	if p.eat("extern") {
		ft.Abi = "C"
		if p.cur().Kind == lexer.Str {
			ft.Abi = p.next().Text
		}
	}
	p.expect("fn")
	p.expect("(")
	for !p.is(")") {
		p.parseOuterAttrs()
		if p.eat("...") {
			ft.Variadic = true
			break
		}
		t := p.cur()
		if (isPlainIdent(t) || t.Is("_")) && p.isN(1, ":") && !p.isN(1, "::") {
			p.next()
			p.next()
			if p.eat("...") {
				ft.Variadic = true
				break
			}
		}
		ft.Inputs = append(ft.Inputs, p.parseType())
		if !p.eat(",") {
			break
		}
	}
	p.eat(",")
	p.expect(")")
	if p.eat("->") {
		ft.Output = p.parseTypeNoPlus()
	}
	ft.TypeBase = p.tb(lo)
	return ft
}

// ---------------------------------------------------------------------------
// Bounds and generics

func (p *Parser) parseForLifetimes() []string {
	p.expect("for")
	p.expectPrefix("<")
	var lts []string
	for !p.isGt() {
		p.parseOuterAttrs()
		t := p.cur()
		if t.Kind != lexer.Lifetime {
			p.errExpected("lifetime")
		}
		p.next()
		lts = append(lts, t.Text)
		if p.eat(":") {
			// bounds on HRTB lifetimes are rare; skip them
			for p.cur().Kind == lexer.Lifetime {
				p.next()
				if !p.eat("+") {
					break
				}
			}
		}
		if !p.eat(",") {
			break
		}
	}
	p.expectPrefix(">")
	return lts
}

func (p *Parser) canBeginBound() bool {
	t := p.cur()
	if t.Kind == lexer.Lifetime {
		return true
	}
	return t.Is("?") || t.Is("~") || t.Is("(") || t.Is("for") || t.Is("::") || t.Is("<") || t.Is("const") || isPathSegStart(t)
}

// parseBounds parses `A + B + 'a`. With plus == false only one bound is
// parsed.
func (p *Parser) parseBounds(plus bool) []*ast.Bound {
	var bounds []*ast.Bound
	if p.skipUseBound() {
		if !plus || !p.is("+") {
			return nil
		}
		return p.parseMoreBounds(nil)
	}
	if !p.canBeginBound() {
		return nil
	}
	bounds = append(bounds, p.parseBound())
	if plus {
		bounds = p.parseMoreBounds(bounds)
	}
	return bounds
}

func (p *Parser) parseMoreBounds(bounds []*ast.Bound) []*ast.Bound {
	for p.is("+") {
		p.next()
		if p.skipUseBound() {
			continue
		}
		if !p.canBeginBound() {
			break
		}
		bounds = append(bounds, p.parseBound())
	}
	return bounds
}

// skipUseBound skips a precise-capturing bound `use<'a, T, N>` (only
// meaningful on `impl Trait`; captures are not tracked, so it is dropped).
func (p *Parser) skipUseBound() bool {
	if !p.is("use") || !p.peekN(1).Is("<") {
		return false
	}
	p.next()
	p.expectPrefix("<")
	for !p.isGt() && p.cur().Kind != lexer.EOF {
		p.next()
		if !p.eat(",") {
			break
		}
	}
	p.expectPrefix(">")
	return true
}

func (p *Parser) parseBound() *ast.Bound {
	lo := p.lo()
	if p.cur().Kind == lexer.Lifetime {
		t := p.next()
		return &ast.Bound{Lifetime: t.Text, Span: t.Span}
	}
	paren := false
	if p.is("(") {
		p.next()
		paren = true
	}
	b := &ast.Bound{}
	if p.is("for") {
		b.ForLifetimes = p.parseForLifetimes()
	}
	if p.eat("?") {
		b.Maybe = true
	} else if p.is("~") && p.isN(1, "const") {
		p.next()
		p.next()
		b.MaybeConst = true
	} else if p.is("const") {
		p.next()
		b.MaybeConst = true
	}
	if p.is("for") {
		b.ForLifetimes = append(b.ForLifetimes, p.parseForLifetimes()...)
	}
	b.Path = p.parsePath(styleType)
	if paren {
		p.expect(")")
	}
	b.Span = p.span(lo)
	return b
}

// parseGenericParams parses `<...>` generic parameters if present; it always
// returns a non-nil Generics.
func (p *Parser) parseGenericParams() *ast.Generics {
	g := &ast.Generics{}
	lo := p.lo()
	if !p.isPrefix("<") || p.is("<=") || p.is("<<=") {
		g.Span = lexer.Span{Lo: lo, Hi: lo}
		return g
	}
	p.expectPrefix("<")
	for !p.isGt() {
		attrs := p.parseOuterAttrs()
		gp := &ast.GenericParam{Attrs: attrs, ID: ast.NewID()}
		t := p.cur()
		switch {
		case t.Kind == lexer.Lifetime:
			p.next()
			gp.Kind = ast.GPLifetime
			gp.Name = ast.Ident{Name: t.Text, Span: t.Span, Hyg: t.Hyg}
			if p.eat(":") {
				for p.cur().Kind == lexer.Lifetime {
					lt := p.next()
					gp.Bounds = append(gp.Bounds, &ast.Bound{Lifetime: lt.Text, Span: lt.Span})
					if !p.eat("+") {
						break
					}
				}
			}
		case t.Is("const"):
			p.next()
			gp.Kind = ast.GPConst
			gp.Name = p.parseIdentTok()
			p.expect(":")
			gp.ConstTy = p.parseType()
			if p.eat("=") {
				gp.ConstDefault = p.parseConstArgExpr()
			}
		default:
			gp.Kind = ast.GPType
			gp.Name = p.parseIdentTok()
			if p.eat(":") {
				gp.Bounds = p.parseBounds(true)
			}
			if p.eat("=") {
				gp.Default = p.parseType()
			}
		}
		g.Params = append(g.Params, gp)
		if !p.eat(",") {
			break
		}
	}
	p.expectPrefix(">")
	g.Span = p.span(lo)
	return g
}

// parseWhere parses an optional where clause into g.
func (p *Parser) parseWhere(g *ast.Generics) {
	if !p.eat("where") {
		return
	}
	for {
		if p.is("{") || p.is(";") || p.is("=") || p.atEOF() {
			return
		}
		lo := p.lo()
		pred := &ast.WherePredicate{}
		if p.is("for") {
			pred.ForLifetimes = p.parseForLifetimes()
		}
		if t := p.cur(); t.Kind == lexer.Lifetime {
			p.next()
			pred.Lifetime = t.Text
			p.expect(":")
			for p.cur().Kind == lexer.Lifetime {
				lt := p.next()
				pred.Bounds = append(pred.Bounds, &ast.Bound{Lifetime: lt.Text, Span: lt.Span})
				if !p.eat("+") {
					break
				}
			}
		} else {
			pred.Ty = p.parseTypeNoPlus()
			p.expect(":")
			pred.Bounds = p.parseBounds(true)
		}
		pred.Span = p.span(lo)
		g.Where = append(g.Where, pred)
		if !p.eat(",") {
			return
		}
	}
}

// gatBindingAhead reports whether `Ident < ... >` at the cursor is followed
// by `=` or `:` (a generic associated type binding, not a type argument).
func (p *Parser) gatBindingAhead() bool {
	depth := 0
	for i := 1; i < 256; i++ {
		t := p.peekN(i)
		switch {
		case t.Kind == lexer.EOF:
			return false
		case t.Is("<"):
			depth++
		case t.Is(">"):
			depth--
		case t.Is(">>"):
			depth -= 2
		case t.Is("->"), t.Is(">="), t.Is(">>="):
			// not plausible inside the GAT args of a binding
		case t.Is(";"), t.Is("{"), t.Is("}"):
			return false
		}
		if depth < 0 {
			return false // `>>` also closed the enclosing list
		}
		if depth == 0 {
			n := p.peekN(i + 1)
			return n.Is("=") || (n.Is(":") && !n.Is("::"))
		}
	}
	return false
}
