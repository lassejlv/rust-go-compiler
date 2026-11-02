package parser

import (
	"math/big"
	"strconv"
	"strings"

	"rustgo/compiler/ast"
	"rustgo/compiler/lexer"
)

// restr holds expression restrictions.
type restr uint8

const (
	// rNoStruct forbids struct literals (`Path { ... }`) — used for
	// if/while/match/for heads.
	rNoStruct restr = 1 << iota
)

// Operator precedences (higher binds tighter).
const (
	precAssign = 2
	precRange  = 3
	precOr     = 4
	precAnd    = 5
	precCmp    = 6
	precBitOr  = 7
	precBitXor = 8
	precBitAnd = 9
	precShift  = 10
	precAdd    = 11
	precMul    = 12
	precCast   = 13
)

type opInfo struct {
	prec   int
	op     ast.BinOp
	assign bool // `=` or op=
	opAsg  bool // compound assignment
	rng    bool
	incl   bool
	cast   bool
}

func binOpInfo(t lexer.Token) (opInfo, bool) {
	if t.Kind == lexer.Ident {
		if !t.Raw && t.Text == "as" {
			return opInfo{prec: precCast, cast: true}, true
		}
		return opInfo{}, false
	}
	if t.Kind != lexer.Punct {
		return opInfo{}, false
	}
	switch t.Text {
	case "*":
		return opInfo{prec: precMul, op: ast.BinMul}, true
	case "/":
		return opInfo{prec: precMul, op: ast.BinDiv}, true
	case "%":
		return opInfo{prec: precMul, op: ast.BinRem}, true
	case "+":
		return opInfo{prec: precAdd, op: ast.BinAdd}, true
	case "-":
		return opInfo{prec: precAdd, op: ast.BinSub}, true
	case "<<":
		return opInfo{prec: precShift, op: ast.BinShl}, true
	case ">>":
		return opInfo{prec: precShift, op: ast.BinShr}, true
	case "&":
		return opInfo{prec: precBitAnd, op: ast.BinBitAnd}, true
	case "^":
		return opInfo{prec: precBitXor, op: ast.BinBitXor}, true
	case "|":
		return opInfo{prec: precBitOr, op: ast.BinBitOr}, true
	case "==":
		return opInfo{prec: precCmp, op: ast.BinEq}, true
	case "!=":
		return opInfo{prec: precCmp, op: ast.BinNe}, true
	case "<":
		return opInfo{prec: precCmp, op: ast.BinLt}, true
	case "<=":
		return opInfo{prec: precCmp, op: ast.BinLe}, true
	case ">":
		return opInfo{prec: precCmp, op: ast.BinGt}, true
	case ">=":
		return opInfo{prec: precCmp, op: ast.BinGe}, true
	case "&&":
		return opInfo{prec: precAnd, op: ast.BinAnd}, true
	case "||":
		return opInfo{prec: precOr, op: ast.BinOr}, true
	case "..":
		return opInfo{prec: precRange, rng: true}, true
	case "..=", "...":
		return opInfo{prec: precRange, rng: true, incl: true}, true
	case "=":
		return opInfo{prec: precAssign, assign: true}, true
	}
	if len(t.Text) >= 2 && strings.HasSuffix(t.Text, "=") {
		var op ast.BinOp
		switch t.Text[:len(t.Text)-1] {
		case "+":
			op = ast.BinAdd
		case "-":
			op = ast.BinSub
		case "*":
			op = ast.BinMul
		case "/":
			op = ast.BinDiv
		case "%":
			op = ast.BinRem
		case "^":
			op = ast.BinBitXor
		case "&":
			op = ast.BinBitAnd
		case "|":
			op = ast.BinBitOr
		case "<<":
			op = ast.BinShl
		case ">>":
			op = ast.BinShr
		default:
			return opInfo{}, false
		}
		return opInfo{prec: precAssign, assign: true, opAsg: true, op: op}, true
	}
	return opInfo{}, false
}

// keywords that can never begin an expression
var nonExprKeywords = map[string]bool{
	"as": true, "else": true, "in": true, "where": true, "fn": true, "struct": true,
	"enum": true, "trait": true, "impl": true, "type": true, "use": true, "mod": true,
	"pub": true, "extern": true, "ref": true, "dyn": true,
}

func (p *Parser) canBeginExpr(r restr) bool {
	t := p.cur()
	switch t.Kind {
	case lexer.EOF:
		return false
	case lexer.Ident:
		return t.Raw || !nonExprKeywords[t.Text]
	case lexer.Punct:
		switch t.Text {
		case "(", "[", "-", "!", "*", "&", "&&", "|", "||", "..", "..=", "<", "<<", "::", "#", "$":
			return true
		case "{":
			return r&rNoStruct == 0
		}
		return false
	}
	return true
}

func (p *Parser) parseExpr() ast.Expr { return p.parseAssoc(0, 0) }

func (p *Parser) parseExprR(r restr) ast.Expr { return p.parseAssoc(0, r) }

func (p *Parser) parseExprNoStruct() ast.Expr { return p.parseAssoc(0, rNoStruct) }

func (p *Parser) parseAssoc(minPrec int, r restr) ast.Expr {
	lo := p.lo()
	var lhs ast.Expr
	if p.is("..") || p.is("..=") || p.is("...") {
		incl := !p.is("..")
		p.next()
		var end ast.Expr
		if incl || p.canBeginExpr(r) {
			end = p.parseAssoc(precRange+1, r)
		}
		lhs = &ast.RangeExpr{ExprBase: p.eb(lo), End: end, Inclusive: incl}
	} else {
		lhs = p.parseUnary(r)
	}
	return p.parseAssocRest(lhs, lo, minPrec, r)
}

func (p *Parser) parseAssocRest(lhs ast.Expr, lo lexer.Pos, minPrec int, r restr) ast.Expr {
	for {
		info, ok := binOpInfo(p.cur())
		if !ok || info.prec < minPrec {
			return lhs
		}
		p.next()
		switch {
		case info.cast:
			ty := p.parseCastType()
			lhs = &ast.CastExpr{ExprBase: p.eb(lo), X: lhs, Ty: ty}
		case info.rng:
			var end ast.Expr
			if info.incl || p.canBeginExpr(r) {
				end = p.parseAssoc(precRange+1, r)
			}
			lhs = &ast.RangeExpr{ExprBase: p.eb(lo), Start: lhs, End: end, Inclusive: info.incl}
		case info.assign:
			rhs := p.parseAssoc(precAssign, r)
			if info.opAsg {
				lhs = &ast.AssignOpExpr{ExprBase: p.eb(lo), Op: info.op, L: lhs, R: rhs}
			} else {
				lhs = &ast.AssignExpr{ExprBase: p.eb(lo), L: lhs, R: rhs}
			}
		default:
			rhs := p.parseAssoc(info.prec+1, r)
			lhs = &ast.BinaryExpr{ExprBase: p.eb(lo), Op: info.op, L: lhs, R: rhs}
		}
	}
}

// parseCastType parses the type after `as`. `x as usize < y` is accepted by
// retrying without generic arguments when the generic parse fails.
func (p *Parser) parseCastType() (ty ast.Type) {
	save := p.Pos()
	saveLast := p.lastHi
	ok := func() (ok bool) {
		defer func() {
			if r := recover(); r != nil {
				if _, isPE := r.(parseError); !isPE {
					panic(r)
				}
				ok = false
			}
		}()
		ty = p.parseTypeNoPlus()
		return true
	}()
	if ok {
		return ty
	}
	p.Reset(save)
	p.lastHi = saveLast
	old := p.noTypeLt
	p.noTypeLt = true
	ty = p.parseTypeNoPlus()
	p.noTypeLt = old
	return ty
}

func setExprAttrs(e ast.Expr, attrs []*ast.Attr) {
	if len(attrs) == 0 {
		return
	}
	if b, ok := e.(interface{ BaseExpr() *ast.ExprBase }); ok {
		eb := b.BaseExpr()
		eb.Attrs = append(attrs, eb.Attrs...)
	}
}

func (p *Parser) parseUnary(r restr) ast.Expr {
	lo := p.lo()
	t := p.cur()
	if p.isOuterAttrStart() {
		attrs := p.parseOuterAttrs()
		e := p.parseUnary(r)
		setExprAttrs(e, attrs)
		return e
	}
	if t.Kind == lexer.Punct {
		switch t.Text {
		case "-":
			p.next()
			x := p.parseUnary(r)
			return &ast.UnaryExpr{ExprBase: p.eb(lo), Op: ast.UnNeg, X: x}
		case "!":
			p.next()
			x := p.parseUnary(r)
			return &ast.UnaryExpr{ExprBase: p.eb(lo), Op: ast.UnNot, X: x}
		case "*":
			p.next()
			x := p.parseUnary(r)
			return &ast.UnaryExpr{ExprBase: p.eb(lo), Op: ast.UnDeref, X: x}
		case "&", "&&":
			double := t.Text == "&&"
			p.next()
			innerLo := lo
			if double {
				innerLo.Off++
				innerLo.Col++
			}
			re := &ast.RefExpr{}
			if p.cur().Kind == lexer.Ident && !p.cur().Raw && p.cur().Text == "raw" && (p.isN(1, "const") || p.isN(1, "mut")) {
				p.next()
				re.Raw = true
				re.Mut = p.next().Text == "mut"
			} else if p.eat("mut") {
				re.Mut = true
			}
			re.X = p.parseUnary(r)
			re.ExprBase = p.eb(innerLo)
			if double {
				return &ast.RefExpr{ExprBase: p.eb(lo), X: re}
			}
			return re
		}
	}
	if t.Is("box") && p.canBeginExprAt(1) {
		p.errorf("`box` expressions are not supported")
	}
	e := p.parsePrimary(r)
	return p.parsePostfix(e, lo)
}

func (p *Parser) canBeginExprAt(n int) bool {
	t := p.peekN(n)
	return t.Kind != lexer.EOF && !(t.Kind == lexer.Punct && strings.Contains(");,]}=>", t.Text))
}

func (p *Parser) parsePostfix(e ast.Expr, lo lexer.Pos) ast.Expr {
	for {
		t := p.cur()
		if t.Kind != lexer.Punct {
			return e
		}
		switch t.Text {
		case "?":
			p.next()
			e = &ast.TryExpr{ExprBase: p.eb(lo), X: e}
		case "(":
			p.next()
			args := p.parseExprList(")")
			e = &ast.CallExpr{ExprBase: p.eb(lo), Fn: e, Args: args}
		case "[":
			p.next()
			idx := p.parseExpr()
			p.expect("]")
			e = &ast.IndexExpr{ExprBase: p.eb(lo), X: e, Index: idx}
		case ".":
			p.next()
			e = p.parseDotSuffix(e, lo)
		default:
			return e
		}
	}
}

func (p *Parser) parseDotSuffix(e ast.Expr, lo lexer.Pos) ast.Expr {
	t := p.cur()
	switch t.Kind {
	case lexer.Int:
		p.next()
		return &ast.FieldExpr{ExprBase: p.eb(lo), X: e, Name: ast.Ident{Name: t.Text, Span: t.Span, Hyg: t.Hyg}}
	case lexer.Float:
		// `x.0.1` lexed as a float `0.1`
		p.next()
		parts := strings.Split(t.Text, ".")
		for _, part := range parts {
			if part == "" {
				continue
			}
			if _, err := strconv.Atoi(part); err != nil {
				panic(parseError{&Error{Pos: t.Span.Lo, Msg: "invalid tuple index `" + t.Text + "`"}})
			}
			e = &ast.FieldExpr{ExprBase: p.eb(lo), X: e, Name: ast.Ident{Name: part, Span: t.Span}}
		}
		return e
	case lexer.Ident:
		if t.Is("await") {
			p.next()
			return &ast.AwaitExpr{ExprBase: p.eb(lo), X: e}
		}
		p.next()
		name := ast.Ident{Name: t.Text, Span: t.Span, Hyg: t.Hyg}
		var gargs *ast.GenericArgs
		if p.is("::") {
			p.next()
			if !p.isPrefix("<") {
				p.errExpected("`<`")
			}
			gargs = p.parseGenericArgs()
		}
		if p.is("(") {
			p.next()
			args := p.parseExprList(")")
			return &ast.MethodCallExpr{ExprBase: p.eb(lo), Receiver: e, Method: name, Generics: gargs, Args: args}
		}
		if gargs != nil {
			p.errExpected("`(`")
		}
		return &ast.FieldExpr{ExprBase: p.eb(lo), X: e, Name: name}
	}
	p.errExpected("field name or method")
	return nil
}

// parseExprList parses comma-separated expressions up to (and including)
// the closing token.
func (p *Parser) parseExprList(closer string) []ast.Expr {
	var out []ast.Expr
	for !p.is(closer) {
		out = append(out, p.parseExpr())
		if !p.eat(",") {
			break
		}
	}
	p.expect(closer)
	return out
}

func (p *Parser) parsePrimary(r restr) ast.Expr {
	lo := p.lo()
	t := p.cur()
	switch t.Kind {
	case lexer.Int, lexer.Float, lexer.Str, lexer.ByteStr, lexer.CStr, lexer.Char, lexer.Byte:
		return p.parseLit()
	case lexer.Lifetime:
		label := t.Text
		p.next()
		p.expect(":")
		switch {
		case p.is("loop"):
			return p.parseLoop(lo, label)
		case p.is("while"):
			return p.parseWhile(lo, label)
		case p.is("for"):
			return p.parseFor(lo, label)
		case p.is("{"):
			b := p.parseBlock()
			return &ast.BlockExpr{ExprBase: p.eb(lo), Block: b, Label: label}
		case p.is("unsafe") && p.isN(1, "{"):
			p.next()
			b := p.parseBlock()
			b.Unsafe = true
			return &ast.BlockExpr{ExprBase: p.eb(lo), Block: b, Label: label}
		}
		p.errExpected("`loop`, `while`, `for` or block after label")
	case lexer.Ident:
		if t.Raw {
			return p.parsePathStartExpr(r)
		}
		switch t.Text {
		case "true", "false":
			return p.parseLit()
		case "if":
			return p.parseIf()
		case "match":
			return p.parseMatch()
		case "loop":
			return p.parseLoop(lo, "")
		case "while":
			return p.parseWhile(lo, "")
		case "for":
			return p.parseFor(lo, "")
		case "unsafe":
			p.next()
			if !p.is("{") {
				p.errExpected("`{`")
			}
			b := p.parseBlock()
			b.Unsafe = true
			return &ast.BlockExpr{ExprBase: p.eb(lo), Block: b}
		case "const":
			if p.isN(1, "{") {
				p.next()
				b := p.parseBlock()
				return &ast.BlockExpr{ExprBase: p.eb(lo), Block: b, Const: true}
			}
		case "async":
			if p.isN(1, "{") || (p.isN(1, "move") && p.isN(2, "{")) {
				p.next()
				mv := p.eat("move")
				b := p.parseBlock()
				return &ast.BlockExpr{ExprBase: p.eb(lo), Block: b, Async: true, Move: mv}
			}
			return p.parseClosure(r)
		case "move", "static":
			return p.parseClosure(r)
		case "return":
			p.next()
			var v ast.Expr
			if p.canBeginExpr(r) {
				v = p.parseExprR(r)
			}
			return &ast.ReturnExpr{ExprBase: p.eb(lo), Value: v}
		case "break":
			p.next()
			be := &ast.BreakExpr{}
			if p.cur().Kind == lexer.Lifetime {
				be.Label = p.next().Text
			}
			if p.canBeginExpr(r) {
				be.Value = p.parseExprR(r)
			}
			be.ExprBase = p.eb(lo)
			return be
		case "continue":
			p.next()
			ce := &ast.ContinueExpr{}
			if p.cur().Kind == lexer.Lifetime {
				ce.Label = p.next().Text
			}
			ce.ExprBase = p.eb(lo)
			return ce
		case "let":
			p.next()
			pat := p.parsePat()
			p.expect("=")
			scrut := p.parseAssoc(precCmp, r)
			return &ast.LetExpr{ExprBase: p.eb(lo), Pat: pat, Expr: scrut}
		case "_":
			p.next()
			return &ast.UnderscoreExpr{ExprBase: p.eb(lo)}
		}
		if isPathSegStart(t) {
			return p.parsePathStartExpr(r)
		}
	case lexer.Punct:
		switch t.Text {
		case "(":
			return p.parseParenExpr()
		case "[":
			return p.parseArrayExpr()
		case "{":
			b := p.parseBlock()
			return &ast.BlockExpr{ExprBase: p.eb(lo), Block: b}
		case "|", "||":
			return p.parseClosure(r)
		case "<", "<<", "::", "$":
			return p.parsePathStartExpr(r)
		}
	}
	p.errExpected("expression")
	return nil
}

func (p *Parser) parseParenExpr() ast.Expr {
	lo := p.lo()
	p.expect("(")
	if p.eat(")") {
		return &ast.TupleExpr{ExprBase: p.eb(lo)}
	}
	first := p.parseExpr()
	if p.eat(")") {
		return &ast.ParenExpr{ExprBase: p.eb(lo), X: first}
	}
	elems := []ast.Expr{first}
	for p.eat(",") {
		if p.is(")") {
			break
		}
		elems = append(elems, p.parseExpr())
	}
	p.expect(")")
	return &ast.TupleExpr{ExprBase: p.eb(lo), Elems: elems}
}

func (p *Parser) parseArrayExpr() ast.Expr {
	lo := p.lo()
	p.expect("[")
	if p.eat("]") {
		return &ast.ArrayExpr{ExprBase: p.eb(lo)}
	}
	first := p.parseExpr()
	if p.eat(";") {
		n := p.parseExpr()
		p.expect("]")
		return &ast.RepeatExpr{ExprBase: p.eb(lo), Elem: first, Count: n}
	}
	elems := []ast.Expr{first}
	for p.eat(",") {
		if p.is("]") {
			break
		}
		elems = append(elems, p.parseExpr())
	}
	p.expect("]")
	return &ast.ArrayExpr{ExprBase: p.eb(lo), Elems: elems}
}

func (p *Parser) parseBlockExpr(label string) *ast.BlockExpr {
	lo := p.lo()
	b := p.parseBlock()
	return &ast.BlockExpr{ExprBase: p.eb(lo), Block: b, Label: label}
}

func (p *Parser) parseIf() ast.Expr {
	lo := p.lo()
	p.expect("if")
	cond := p.parseExprNoStruct()
	then := p.parseBlock()
	ie := &ast.IfExpr{Cond: cond, Then: then}
	if p.eat("else") {
		if p.is("if") {
			ie.Else = p.parseIf()
		} else {
			elo := p.lo()
			b := p.parseBlock()
			ie.Else = &ast.BlockExpr{ExprBase: p.eb(elo), Block: b}
		}
	}
	ie.ExprBase = p.eb(lo)
	return ie
}

func (p *Parser) parseLoop(lo lexer.Pos, label string) ast.Expr {
	p.expect("loop")
	b := p.parseBlock()
	return &ast.LoopExpr{ExprBase: p.eb(lo), Body: b, Label: label}
}

func (p *Parser) parseWhile(lo lexer.Pos, label string) ast.Expr {
	p.expect("while")
	cond := p.parseExprNoStruct()
	b := p.parseBlock()
	return &ast.WhileExpr{ExprBase: p.eb(lo), Cond: cond, Body: b, Label: label}
}

func (p *Parser) parseFor(lo lexer.Pos, label string) ast.Expr {
	p.expect("for")
	pat := p.parsePat()
	p.expect("in")
	iter := p.parseExprNoStruct()
	b := p.parseBlock()
	return &ast.ForExpr{ExprBase: p.eb(lo), Pat: pat, Iter: iter, Body: b, Label: label}
}

func (p *Parser) parseMatch() ast.Expr {
	lo := p.lo()
	p.expect("match")
	scrut := p.parseExprNoStruct()
	p.expect("{")
	p.parseInnerAttrs()
	me := &ast.MatchExpr{Scrutinee: scrut}
	for !p.is("}") {
		me.Arms = append(me.Arms, p.parseArm())
	}
	p.expect("}")
	me.ExprBase = p.eb(lo)
	return me
}

func (p *Parser) parseArm() *ast.Arm {
	lo := p.lo()
	arm := &ast.Arm{Attrs: p.parseOuterAttrs()}
	arm.Pat = p.parsePat()
	if p.eat("if") {
		arm.Guard = p.parseExpr()
	}
	p.expect("=>")
	if p.isBlockLikeStart() {
		blo := p.lo()
		e := p.parseBlockLikeWithPostfix(blo)
		arm.Body = e
		if isBlockLike(e) {
			p.eat(",")
		} else if !p.eat(",") && !p.is("}") {
			p.errExpected("`,` or `}`")
		}
	} else {
		arm.Body = p.parseExpr()
		if !p.eat(",") && !p.is("}") {
			p.errExpected("`,` or `}` after match arm")
		}
	}
	arm.Span = p.span(lo)
	arm.ID = ast.NewID()
	return arm
}

func (p *Parser) parseClosure(r restr) ast.Expr {
	lo := p.lo()
	ce := &ast.ClosureExpr{}
	p.eat("static")
	if p.eat("async") {
		ce.Async = true
	}
	if p.eat("move") {
		ce.Move = true
	}
	if p.eat("||") {
		// no params
	} else {
		p.expect("|")
		for !p.is("|") {
			p.parseOuterAttrs()
			cp := &ast.ClosureParam{Pat: p.parsePatNoTopAlt()}
			if p.eat(":") {
				cp.Ty = p.parseTypeNoPlus()
			}
			ce.Params = append(ce.Params, cp)
			if !p.eat(",") {
				break
			}
		}
		p.expect("|")
	}
	if p.eat("->") {
		ce.Ret = p.parseTypeNoPlus()
		if !p.is("{") {
			p.errExpected("`{` (closure body with a return type must be a block)")
		}
		ce.Body = p.parseBlockExpr("")
	} else {
		ce.Body = p.parseAssoc(0, r)
	}
	ce.ExprBase = p.eb(lo)
	return ce
}

// parsePathStartExpr parses an expression starting with a path: a path
// expression, a struct literal, or a macro call.
func (p *Parser) parsePathStartExpr(r restr) ast.Expr {
	lo := p.lo()
	var qself *ast.QSelf
	var path *ast.Path
	if p.isPrefix("<") {
		qself, path = p.parseQPath(styleExpr)
	} else {
		path = p.parsePath(styleExpr)
	}
	if qself == nil && p.is("!") && (p.isN(1, "(") || p.isN(1, "[") || p.isN(1, "{")) && !pathHasArgs(path) {
		p.next()
		g := p.parseDelimGroup()
		mac := &ast.MacCall{Path: path, Delim: g.Delim, Args: g.Inner, Span: p.span(lo)}
		if isAsmMac(mac) {
			return p.parseAsm(mac, p.eb(lo))
		}
		return &ast.MacCallExpr{ExprBase: p.eb(lo), Mac: mac}
	}
	if p.is("{") && r&rNoStruct == 0 {
		return p.parseStructLit(lo, qself, path)
	}
	return &ast.PathExpr{ExprBase: p.eb(lo), QSelf: qself, Path: path}
}

func (p *Parser) parseStructLit(lo lexer.Pos, qself *ast.QSelf, path *ast.Path) ast.Expr {
	p.expect("{")
	se := &ast.StructExpr{QSelf: qself, Path: path}
	for !p.is("}") {
		fattrs := p.parseOuterAttrs()
		if p.eat("..") {
			if !p.is("}") {
				se.Base = p.parseExpr()
			} else {
				se.Rest = true
			}
			break
		}
		flo := p.lo()
		t := p.cur()
		var name ast.Ident
		switch {
		case t.Kind == lexer.Int:
			p.next()
			name = ast.Ident{Name: t.Text, Span: t.Span, Hyg: t.Hyg}
		case t.Kind == lexer.Ident:
			p.next()
			name = ast.Ident{Name: t.Text, Span: t.Span, Hyg: t.Hyg}
		default:
			p.errExpected("field name")
		}
		fi := &ast.FieldInit{Name: name, Attrs: fattrs}
		if p.eat(":") {
			fi.Expr = p.parseExpr()
		} else {
			fi.Shorthand = true
			fi.Expr = &ast.PathExpr{
				ExprBase: ast.ExprBase{Span: name.Span, ID: ast.NewID()},
				Path: &ast.Path{
					Segments: []*ast.PathSegment{{Ident: name}},
					Span:     name.Span,
					ID:       ast.NewID(),
				},
			}
		}
		fi.Span = p.span(flo)
		se.Fields = append(se.Fields, fi)
		if !p.eat(",") {
			break
		}
	}
	p.expect("}")
	se.ExprBase = p.eb(lo)
	return se
}

// ---------------------------------------------------------------------------
// Literals

func (p *Parser) parseLit() *ast.LitExpr {
	lo := p.lo()
	t := p.cur()
	lit := &ast.LitExpr{Suffix: t.Suffix}
	switch t.Kind {
	case lexer.Int:
		lit.Kind = ast.LitInt
		lit.Value = t.Text
		v, bigText, ok := DecodeInt(t.Text)
		if !ok {
			p.errorf("invalid integer literal `%s`", t.Text)
		}
		lit.IntVal, lit.Big = v, bigText
	case lexer.Float:
		lit.Kind = ast.LitFloat
		v := strings.ReplaceAll(t.Text, "_", "")
		if strings.HasSuffix(v, ".") {
			v += "0"
		}
		lit.Value = v
	case lexer.Str:
		lit.Kind = ast.LitStr
		lit.Value = t.Text
	case lexer.ByteStr:
		lit.Kind = ast.LitByteStr
		lit.Value = t.Text
	case lexer.CStr:
		lit.Kind = ast.LitCStr
		lit.Value = t.Text
	case lexer.Char:
		lit.Kind = ast.LitChar
		lit.Value = t.Text
		for _, r := range t.Text {
			lit.IntVal = uint64(r)
			break
		}
	case lexer.Byte:
		lit.Kind = ast.LitByte
		lit.Value = t.Text
		if len(t.Text) > 0 {
			lit.IntVal = uint64(t.Text[0])
		}
	case lexer.Ident:
		if !t.Raw && (t.Text == "true" || t.Text == "false") {
			lit.Kind = ast.LitBool
			lit.Value = t.Text
			if t.Text == "true" {
				lit.IntVal = 1
			}
			break
		}
		p.errExpected("literal")
	default:
		p.errExpected("literal")
	}
	p.next()
	lit.ExprBase = p.eb(lo)
	return lit
}

// DecodeInt decodes the text of an integer literal (with optional base
// prefix and underscores). For values that do not fit in uint64, big holds
// the decimal text and v the low 64 bits.
func DecodeInt(text string) (v uint64, bigText string, ok bool) {
	s := strings.ReplaceAll(text, "_", "")
	base := 10
	if len(s) > 2 && s[0] == '0' {
		switch s[1] {
		case 'x', 'X':
			base = 16
		case 'o':
			base = 8
		case 'b':
			base = 2
		}
		if base != 10 {
			s = s[2:]
		}
	}
	if s == "" {
		return 0, "", false
	}
	v, err := strconv.ParseUint(s, base, 64)
	if err == nil {
		return v, "", true
	}
	bi, ok2 := new(big.Int).SetString(s, base)
	if !ok2 {
		return 0, "", false
	}
	low := new(big.Int).And(bi, new(big.Int).SetUint64(^uint64(0)))
	return low.Uint64(), bi.String(), true
}
