package parser

import (
	"rustgo/compiler/ast"
	"rustgo/compiler/lexer"
)

func (p *Parser) parseBlock() *ast.Block {
	lo := p.lo()
	p.expect("{")
	stmts := p.parseStmtsUntil(false)
	p.expect("}")
	return &ast.Block{Stmts: stmts, Span: p.span(lo), ID: ast.NewID()}
}

// atBlockEnd reports whether the current token ends a statement list.
func (p *Parser) atBlockEnd() bool { return p.is("}") || p.atEOF() }

// parseStmtsUntil parses statements up to `}` (not consumed) or, with
// toEOF, up to EOF.
func (p *Parser) parseStmtsUntil(toEOF bool) []ast.Stmt {
	p.parseInnerAttrs()
	var stmts []ast.Stmt
	for {
		if p.atEOF() {
			if !toEOF {
				p.errExpected("`}`")
			}
			break
		}
		if p.is("}") {
			if toEOF {
				p.errorf("unexpected `}`")
			}
			break
		}
		if p.eat(";") {
			continue
		}
		stmts = append(stmts, p.parseStmt())
	}
	return stmts
}

// isBlockLikeStart reports whether the current token starts a block-like
// expression (one that can end an expression statement without `;`).
func (p *Parser) isBlockLikeStart() bool {
	t := p.cur()
	if t.Kind == lexer.Lifetime {
		return p.isN(1, ":")
	}
	if t.Is("{") {
		return true
	}
	if t.Kind != lexer.Ident || t.Raw {
		return false
	}
	switch t.Text {
	case "if", "match", "loop", "while", "for":
		return true
	case "unsafe", "const":
		return p.isN(1, "{")
	case "async":
		return p.isN(1, "{") || (p.isN(1, "move") && p.isN(2, "{"))
	}
	return false
}

func isBlockLike(e ast.Expr) bool {
	switch e.(type) {
	case *ast.BlockExpr, *ast.IfExpr, *ast.MatchExpr, *ast.LoopExpr, *ast.WhileExpr, *ast.ForExpr:
		return true
	}
	return false
}

// parseBlockLikeWithPostfix parses a block-like expression; if it is
// followed by `.` or `?` the expression continues as rustc does
// (`match x {}.foo()`), including any binary operators afterwards.
func (p *Parser) parseBlockLikeWithPostfix(lo lexer.Pos) ast.Expr {
	e := p.parsePrimary(0)
	if p.is(".") || p.is("?") {
		e = p.parsePostfix(e, lo)
		e = p.parseAssocRest(e, lo, 0, 0)
	}
	return e
}

func (p *Parser) parseStmt() ast.Stmt {
	attrs := p.parseOuterAttrs()
	lo := p.lo()
	if len(attrs) > 0 {
		lo = attrs[0].Span.Lo
	}
	if p.is("let") {
		return p.parseLet(lo, attrs)
	}
	if p.isItemStart() {
		return &ast.ItemStmt{Item: p.parseItemAfterAttrs(lo, attrs)}
	}
	if p.isMacroCallStart() {
		return p.parseMacStmt(attrs)
	}
	if p.isBlockLikeStart() {
		elo := p.lo()
		e := p.parseBlockLikeWithPostfix(elo)
		setExprAttrs(e, attrs)
		if isBlockLike(e) {
			if p.eat(";") {
				return &ast.ExprStmt{Expr: e, Semi: true}
			}
			end := p.atBlockEnd()
			return &ast.ExprStmt{Expr: e, Semi: !end, ImplicitSemi: !end}
		}
		return p.finishExprStmt(e)
	}
	e := p.parseExpr()
	setExprAttrs(e, attrs)
	return p.finishExprStmt(e)
}

func (p *Parser) finishExprStmt(e ast.Expr) ast.Stmt {
	if p.eat(";") {
		return &ast.ExprStmt{Expr: e, Semi: true}
	}
	if p.atBlockEnd() {
		return &ast.ExprStmt{Expr: e, Semi: false}
	}
	p.errExpected("`;`")
	return nil
}

func (p *Parser) parseLet(lo lexer.Pos, attrs []*ast.Attr) ast.Stmt {
	p.expect("let")
	ls := &ast.LetStmt{Attrs: attrs}
	ls.Pat = p.parsePat()
	if p.eat(":") {
		ls.Ty = p.parseType()
	}
	if p.eat("=") {
		ls.Init = p.parseExpr()
		if p.eat("else") {
			ls.Else = p.parseBlock()
		}
	}
	p.expect(";")
	ls.Span = p.span(lo)
	ls.ID = ast.NewID()
	return ls
}

// isMacroCallStart reports whether the tokens at the current position look
// like `path!(` / `path![` / `path!{`.
func (p *Parser) isMacroCallStart() bool {
	i := 0
	if p.isN(0, "::") {
		i++
	}
	for {
		t := p.peekN(i)
		if t.Is("$") && p.peekN(i+1).Kind == lexer.Ident && p.peekN(i+1).Text == "crate" {
			i += 2
		} else if t.Kind == lexer.Ident && (t.Raw || !lexer.Keywords[t.Text] || t.Text == "self" || t.Text == "super" || t.Text == "crate") {
			i++
		} else {
			return false
		}
		if p.isN(i, "::") {
			i++
			continue
		}
		break
	}
	if !p.isN(i, "!") {
		return false
	}
	n := p.peekN(i + 1)
	return n.Is("(") || n.Is("[") || n.Is("{")
}

func (p *Parser) parseMacCall() *ast.MacCall {
	lo := p.lo()
	path := p.parsePath(styleMod)
	p.expect("!")
	g := p.parseDelimGroup()
	return &ast.MacCall{Path: path, Delim: g.Delim, Args: g.Inner, Span: p.span(lo)}
}

func (p *Parser) parseMacStmt(attrs []*ast.Attr) ast.Stmt {
	lo := p.lo()
	mac := p.parseMacCall()
	if isAsmMac(mac) {
		var e ast.Expr = p.parseAsm(mac, p.eb(lo))
		setExprAttrs(e, attrs)
		return p.finishExprStmt(e)
	}
	if (mac.Delim == '{' && !p.is(".") && !p.is("?")) || p.is(";") || p.atBlockEnd() {
		if p.eat(";") {
			return &ast.MacStmt{Attrs: attrs, Mac: mac, Semi: true}
		}
		if p.atBlockEnd() {
			return &ast.MacStmt{Attrs: attrs, Mac: mac, Semi: false}
		}
		return &ast.MacStmt{Attrs: attrs, Mac: mac, Semi: true, ImplicitSemi: true}
	}
	var e ast.Expr = &ast.MacCallExpr{ExprBase: p.eb(lo), Mac: mac}
	e = p.parsePostfix(e, lo)
	e = p.parseAssocRest(e, lo, 0, 0)
	setExprAttrs(e, attrs)
	return p.finishExprStmt(e)
}

// isItemStart reports whether the current position starts an item (used in
// statement position).
func (p *Parser) isItemStart() bool {
	t := p.cur()
	if t.Kind != lexer.Ident || t.Raw {
		return false
	}
	n1 := p.peekN(1)
	switch t.Text {
	case "fn", "struct", "enum", "trait", "impl", "type", "use", "mod", "pub", "extern":
		return true
	case "static":
		return !(n1.Is("|") || n1.Is("||") || n1.Is("move") || n1.Is("async"))
	case "const":
		return !n1.Is("{") && !n1.Is("|") && !n1.Is("||") && !n1.Is("move")
	case "unsafe":
		return n1.Is("fn") || n1.Is("impl") || n1.Is("trait") || n1.Is("extern") || n1.Is("auto") || n1.Is("mod")
	case "async":
		return n1.Is("fn") || n1.Is("unsafe") || n1.Is("extern")
	case "union":
		return isPlainIdent(n1)
	case "auto":
		return n1.Is("trait")
	case "macro_rules":
		return n1.Is("!") && p.peekN(2).Kind == lexer.Ident
	}
	return false
}
