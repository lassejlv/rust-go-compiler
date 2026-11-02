package parser

import (
	"rustgo/compiler/ast"
	"rustgo/compiler/lexer"
)

// parsePat parses a pattern with optional top-level `|` alternatives (and an
// optional leading `|`).
func (p *Parser) parsePat() ast.Pat {
	lo := p.lo()
	p.eat("|")
	first := p.parsePatNoTopAlt()
	if !p.is("|") {
		return first
	}
	alts := []ast.Pat{first}
	for p.eat("|") {
		alts = append(alts, p.parsePatNoTopAlt())
	}
	return &ast.OrPat{PatBase: p.pb(lo), Alts: alts}
}

// canBeginRangeEnd reports whether the current token can start the upper
// bound of a range pattern.
func (p *Parser) canBeginRangeEnd() bool {
	t := p.cur()
	if isLitTok(t) || t.Is("-") || t.Is("::") || t.Is("<") || t.Is("<<") {
		return true
	}
	if t.Kind == lexer.Ident {
		return t.Raw || !lexer.Keywords[t.Text] || t.Text == "self" || t.Text == "Self" || t.Text == "super" || t.Text == "crate"
	}
	return false
}

func (p *Parser) parseRangeEnd() ast.Expr {
	lo := p.lo()
	if p.is("-") {
		p.next()
		lit := p.parseLit()
		return &ast.UnaryExpr{ExprBase: p.eb(lo), Op: ast.UnNeg, X: lit}
	}
	if isLitTok(p.cur()) {
		return p.parseLit()
	}
	if p.isPrefix("<") {
		qs, path := p.parseQPath(styleExpr)
		return &ast.PathExpr{ExprBase: p.eb(lo), QSelf: qs, Path: path}
	}
	path := p.parsePath(styleExpr)
	return &ast.PathExpr{ExprBase: p.eb(lo), Path: path}
}

// maybeRangePat checks for a range operator after a pattern bound lo.
func (p *Parser) maybeRangePat(lo lexer.Pos, start ast.Expr) (ast.Pat, bool) {
	switch {
	case p.is("..=") || p.is("..."):
		p.next()
		hi := p.parseRangeEnd()
		return &ast.RangePat{PatBase: p.pb(lo), Lo: start, Hi: hi, Inclusive: true}, true
	case p.is(".."):
		p.next()
		var hi ast.Expr
		if p.canBeginRangeEnd() {
			hi = p.parseRangeEnd()
		}
		return &ast.RangePat{PatBase: p.pb(lo), Lo: start, Hi: hi}, true
	}
	return nil, false
}

func (p *Parser) parsePatNoTopAlt() ast.Pat {
	lo := p.lo()
	t := p.cur()
	switch t.Kind {
	case lexer.Int, lexer.Float, lexer.Str, lexer.ByteStr, lexer.CStr, lexer.Char, lexer.Byte:
		lit := p.parseLit()
		if rp, ok := p.maybeRangePat(lo, lit); ok {
			return rp
		}
		return &ast.LitPat{PatBase: p.pb(lo), Expr: lit}
	case lexer.Punct:
		switch t.Text {
		case "-":
			p.next()
			lit := p.parseLit()
			neg := &ast.UnaryExpr{ExprBase: p.eb(lo), Op: ast.UnNeg, X: lit}
			if rp, ok := p.maybeRangePat(lo, neg); ok {
				return rp
			}
			return &ast.LitPat{PatBase: p.pb(lo), Expr: neg}
		case "..":
			p.next()
			if p.canBeginRangeEnd() {
				hi := p.parseRangeEnd()
				return &ast.RangePat{PatBase: p.pb(lo), Hi: hi}
			}
			return &ast.RestPat{PatBase: p.pb(lo)}
		case "..=", "...":
			p.next()
			hi := p.parseRangeEnd()
			return &ast.RangePat{PatBase: p.pb(lo), Hi: hi, Inclusive: true}
		case "&", "&&":
			double := t.Text == "&&"
			p.eatPrefix("&")
			if double {
				// The second `&` is still pending; parse it as an inner ref pattern.
				inner := p.parsePatNoTopAlt()
				return &ast.RefPat{PatBase: p.pb(lo), Pat: inner}
			}
			mut := p.eat("mut")
			sub := p.parsePatNoTopAlt()
			return &ast.RefPat{PatBase: p.pb(lo), Mut: mut, Pat: sub}
		case "(":
			p.next()
			var elems []ast.Pat
			trailingComma := false
			for !p.is(")") {
				elems = append(elems, p.parsePat())
				trailingComma = false
				if !p.eat(",") {
					break
				}
				trailingComma = true
			}
			p.expect(")")
			if len(elems) == 1 && !trailingComma {
				if _, isRest := elems[0].(*ast.RestPat); !isRest {
					return &ast.ParenPat{PatBase: p.pb(lo), Pat: elems[0]}
				}
			}
			return &ast.TuplePat{PatBase: p.pb(lo), Elems: elems}
		case "[":
			p.next()
			var elems []ast.Pat
			for !p.is("]") {
				elems = append(elems, p.parsePat())
				if !p.eat(",") {
					break
				}
			}
			p.expect("]")
			return &ast.SlicePat{PatBase: p.pb(lo), Elems: elems}
		case "<", "<<":
			qs, path := p.parseQPath(styleExpr)
			return p.parsePathPatRest(lo, qs, path)
		case "::", "$":
			path := p.parsePath(styleExpr)
			return p.parsePathPatRest(lo, nil, path)
		}
	case lexer.Ident:
		if !t.Raw {
			switch t.Text {
			case "_":
				p.next()
				return &ast.WildPat{PatBase: p.pb(lo)}
			case "true", "false":
				lit := p.parseLit()
				return &ast.LitPat{PatBase: p.pb(lo), Expr: lit}
			case "ref", "mut":
				return p.parseBindingPat(lo)
			case "box":
				p.next()
				sub := p.parsePatNoTopAlt()
				return &ast.BoxPat{PatBase: p.pb(lo), Pat: sub}
			}
		}
		if isPlainIdent(t) {
			n := p.peekN(1)
			if !(n.Is("::") || n.Is("(") || n.Is("{") || n.Is("!") || n.Is("..") || n.Is("..=") || n.Is("...")) {
				return p.parseBindingPat(lo)
			}
		}
		if isPathSegStart(t) {
			path := p.parsePath(styleExpr)
			return p.parsePathPatRest(lo, nil, path)
		}
	}
	p.errExpected("pattern")
	return nil
}

func (p *Parser) parseBindingPat(lo lexer.Pos) ast.Pat {
	ip := &ast.IdentPat{}
	if p.eat("ref") {
		ip.ByRef = true
	}
	if p.eat("mut") {
		ip.Mut = true
	}
	ip.Name = p.parseIdentTok()
	if p.eat("@") {
		ip.Sub = p.parsePatNoTopAlt()
	}
	ip.PatBase = p.pb(lo)
	return ip
}

func (p *Parser) parsePathPatRest(lo lexer.Pos, qs *ast.QSelf, path *ast.Path) ast.Pat {
	switch {
	case qs == nil && p.is("!") && (p.isN(1, "(") || p.isN(1, "[") || p.isN(1, "{")):
		p.next()
		g := p.parseDelimGroup()
		mac := &ast.MacCall{Path: path, Delim: g.Delim, Args: g.Inner, Span: p.span(lo)}
		return &ast.MacPat{PatBase: p.pb(lo), Mac: mac}
	case p.is("("):
		p.next()
		var elems []ast.Pat
		for !p.is(")") {
			elems = append(elems, p.parsePat())
			if !p.eat(",") {
				break
			}
		}
		p.expect(")")
		return &ast.TupleStructPat{PatBase: p.pb(lo), QSelf: qs, Path: path, Elems: elems}
	case p.is("{"):
		return p.parseStructPat(lo, qs, path)
	}
	pe := &ast.PathExpr{ExprBase: p.eb(lo), QSelf: qs, Path: path}
	if rp, ok := p.maybeRangePat(lo, pe); ok {
		return rp
	}
	if qs == nil && !path.Global && len(path.Segments) == 1 && path.Segments[0].Args == nil {
		name := path.Segments[0].Ident
		if name.Name != "self" && name.Name != "super" && name.Name != "crate" && name.Name != "Self" && name.Name != "$crate" {
			return &ast.IdentPat{PatBase: p.pb(lo), Name: name}
		}
	}
	return &ast.PathPat{PatBase: p.pb(lo), QSelf: qs, Path: path}
}

func (p *Parser) parseStructPat(lo lexer.Pos, qs *ast.QSelf, path *ast.Path) ast.Pat {
	p.expect("{")
	sp := &ast.StructPat{QSelf: qs, Path: path}
	for !p.is("}") {
		p.parseOuterAttrs()
		if p.eat("..") {
			sp.Rest = true
			break
		}
		flo := p.lo()
		t := p.cur()
		fp := &ast.FieldPat{}
		switch {
		case t.Is("ref") || t.Is("mut"):
			bp := p.parseBindingPat(flo).(*ast.IdentPat)
			fp.Name = bp.Name
			fp.Pat = bp
			fp.Shorthand = true
		case t.Is("box"):
			p.next()
			bp := p.parseBindingPat(p.lo()).(*ast.IdentPat)
			fp.Name = bp.Name
			fp.Pat = &ast.BoxPat{PatBase: p.pb(flo), Pat: bp}
			fp.Shorthand = true
		case t.Kind == lexer.Int:
			p.next()
			fp.Name = ast.Ident{Name: t.Text, Span: t.Span, Hyg: t.Hyg}
			p.expect(":")
			fp.Pat = p.parsePat()
		case t.Kind == lexer.Ident:
			p.next()
			fp.Name = ast.Ident{Name: t.Text, Span: t.Span, Hyg: t.Hyg}
			if p.eat(":") {
				fp.Pat = p.parsePat()
			} else {
				fp.Shorthand = true
				fp.Pat = &ast.IdentPat{PatBase: ast.PatBase{Span: t.Span, ID: ast.NewID()}, Name: fp.Name}
			}
		default:
			p.errExpected("field pattern")
		}
		fp.Span = p.span(flo)
		sp.Fields = append(sp.Fields, fp)
		if !p.eat(",") {
			break
		}
	}
	p.expect("}")
	sp.PatBase = p.pb(lo)
	return sp
}
