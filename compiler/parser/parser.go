// Package parser is a recursive-descent / Pratt parser for Rust 2021 that
// produces the AST defined in package ast.
package parser

import (
	"fmt"
	"strings"

	"rustgo/compiler/ast"
	"rustgo/compiler/lexer"
)

// Parser parses a token stream. It can be created from source text
// (New / ParseCrate) or from token trees (NewFromTokenTrees, for macros).
type Parser struct {
	toks []lexer.Token // always ends with an EOF token
	idx  int
	// split is the number of bytes of toks[idx].Text that were already
	// consumed (used to split `>>`, `&&`, `||`, `<<`, ... into pieces).
	split  int
	lastHi lexer.Pos

	// noTypeLt disables `<...>` generic args on type paths (used when
	// retrying `x as usize < y`).
	noTypeLt bool
}

// parseError is the panic payload used for error propagation inside the
// parser; it is recovered by the exported entry points.
type parseError struct{ err error }

// Error is a parse error.
type Error struct {
	Pos lexer.Pos
	Msg string
}

func (e *Error) Error() string { return fmt.Sprintf("%s: %s", e.Pos, e.Msg) }

// New tokenizes src and returns a parser positioned at its first token.
func New(file int, src string) (*Parser, error) {
	toks, err := lexer.Tokenize(file, src)
	if err != nil {
		return nil, err
	}
	return NewFromTokens(toks), nil
}

// NewFromTokens returns a parser over toks. An EOF token is appended if the
// slice does not already end with one.
func NewFromTokens(toks []lexer.Token) *Parser {
	// fast path: no doc comments to rewrite and no interior EOF — use the
	// slice as is (the parser never mutates its tokens)
	clean := true
	for i, t := range toks {
		if t.Kind == lexer.DocComment || (t.Kind == lexer.EOF && i != len(toks)-1) {
			clean = false
			break
		}
	}
	if clean && len(toks) > 0 {
		ts := toks
		if ts[len(ts)-1].Kind != lexer.EOF {
			// toks may alias a larger array: never write past its length
			hi := ts[len(ts)-1].Span.Hi
			ts = append(ts[:len(ts):len(ts)], lexer.Token{Kind: lexer.EOF, Span: lexer.Span{Lo: hi, Hi: hi}})
		}
		p := &Parser{toks: ts}
		p.lastHi = ts[0].Span.Lo
		return p
	}
	ts := make([]lexer.Token, 0, len(toks)+1)
	for _, t := range toks {
		if t.Kind == lexer.DocComment {
			// Doc comments are converted into `#[doc = "..."]` attributes.
			ts = append(ts, docCommentTokens(t)...)
			continue
		}
		if t.Kind == lexer.EOF {
			break
		}
		ts = append(ts, t)
	}
	eof := lexer.Token{Kind: lexer.EOF}
	if len(ts) > 0 {
		hi := ts[len(ts)-1].Span.Hi
		eof.Span = lexer.Span{Lo: hi, Hi: hi}
	} else if len(toks) > 0 {
		eof.Span = toks[len(toks)-1].Span
	}
	ts = append(ts, eof)
	p := &Parser{toks: ts}
	p.lastHi = ts[0].Span.Lo
	return p
}

func docCommentTokens(t lexer.Token) []lexer.Token {
	sp := t.Span
	mk := func(k lexer.Kind, s string) lexer.Token { return lexer.Token{Kind: k, Text: s, Span: sp} }
	out := []lexer.Token{mk(lexer.Punct, "#")}
	if t.Inner {
		out = append(out, mk(lexer.Punct, "!"))
	}
	out = append(out, mk(lexer.Punct, "["), mk(lexer.Ident, "doc"), mk(lexer.Punct, "="), mk(lexer.Str, t.Text), mk(lexer.Punct, "]"))
	return out
}

// NewFromTokenTrees returns a parser over the flattened token trees.
func NewFromTokenTrees(tts []ast.TokenTree) *Parser {
	toks := FlattenTokenTrees(tts)
	if len(toks) > 0 {
		// the flattened slice is ours and has room for the EOF
		hi := toks[len(toks)-1].Span.Hi
		toks = append(toks, lexer.Token{Kind: lexer.EOF, Span: lexer.Span{Lo: hi, Hi: hi}})
	}
	return NewFromTokens(toks)
}

// FlattenTokenTrees turns token trees back into a flat token list
// (delimiters included). No EOF token is appended.
func FlattenTokenTrees(tts []ast.TokenTree) []lexer.Token {
	n := 0
	var count func([]ast.TokenTree)
	count = func(ts []ast.TokenTree) {
		for _, tt := range ts {
			if tt.Delim == 0 {
				n++
			} else {
				n += 2
				count(tt.Inner)
			}
		}
	}
	count(tts)
	out := make([]lexer.Token, 0, n+1)
	var rec func([]ast.TokenTree)
	rec = func(ts []ast.TokenTree) {
		for _, tt := range ts {
			if tt.Delim == 0 {
				out = append(out, tt.Tok)
				continue
			}
			out = append(out, lexer.Token{Kind: lexer.Punct, Text: string(tt.Delim), Span: tt.Open})
			rec(tt.Inner)
			out = append(out, lexer.Token{Kind: lexer.Punct, Text: string(closerOf(tt.Delim)), Span: tt.Close})
		}
	}
	rec(tts)
	return out
}

func closerOf(d byte) byte {
	switch d {
	case '(':
		return ')'
	case '[':
		return ']'
	case '{':
		return '}'
	}
	return 0
}

// TokenTrees groups a token list into token trees. EOF tokens are ignored.
func TokenTrees(toks []lexer.Token) ([]ast.TokenTree, error) {
	i := 0
	var rec func(closer string) ([]ast.TokenTree, lexer.Span, error)
	rec = func(closer string) ([]ast.TokenTree, lexer.Span, error) {
		var out []ast.TokenTree
		for i < len(toks) {
			t := toks[i]
			if t.Kind == lexer.EOF {
				break
			}
			if t.Kind == lexer.Punct {
				switch t.Text {
				case "(", "[", "{":
					i++
					inner, cl, err := rec(string(closerOf(t.Text[0])))
					if err != nil {
						return nil, cl, err
					}
					out = append(out, ast.TokenTree{Delim: t.Text[0], Open: t.Span, Close: cl, Inner: inner})
					continue
				case ")", "]", "}":
					if t.Text != closer {
						return nil, t.Span, &Error{Pos: t.Span.Lo, Msg: fmt.Sprintf("unexpected closing delimiter `%s`", t.Text)}
					}
					i++
					return out, t.Span, nil
				}
			}
			out = append(out, ast.TokenTree{Tok: t})
			i++
		}
		if closer != "" {
			var pos lexer.Pos
			if len(toks) > 0 {
				pos = toks[len(toks)-1].Span.Hi
			}
			return nil, lexer.Span{}, &Error{Pos: pos, Msg: fmt.Sprintf("unclosed delimiter: expected `%s`", closer)}
		}
		return out, lexer.Span{}, nil
	}
	out, _, err := rec("")
	return out, err
}

// TokenizeToTrees tokenizes src and groups the tokens into token trees.
func TokenizeToTrees(file int, src string) ([]ast.TokenTree, error) {
	toks, err := lexer.Tokenize(file, src)
	if err != nil {
		return nil, err
	}
	return TokenTrees(toks)
}

// ParseCrate tokenizes and parses a whole source file.
func ParseCrate(file int, src string) (c *ast.Crate, err error) {
	p, err := New(file, src)
	if err != nil {
		return nil, err
	}
	defer p.catch(&err)
	lo := p.cur().Span.Lo
	c = &ast.Crate{}
	c.Attrs = p.parseInnerAttrs()
	for !p.atEOF() {
		if p.eat(";") {
			continue
		}
		c.Items = append(c.Items, p.parseItem())
	}
	c.Span = lexer.Span{Lo: lo, Hi: p.lastHi}
	return c, nil
}

// ---------------------------------------------------------------------------
// Token access

func (p *Parser) cur() lexer.Token {
	t := p.toks[p.idx]
	if p.split > 0 {
		t.Text = t.Text[p.split:]
		t.Span.Lo.Off += p.split
		t.Span.Lo.Col += p.split
	}
	return t
}

// peekN returns the token n positions ahead (n=0 is the current token).
func (p *Parser) peekN(n int) lexer.Token {
	if n == 0 {
		return p.cur()
	}
	i := p.idx + n
	if i >= len(p.toks) {
		i = len(p.toks) - 1
	}
	return p.toks[i]
}

func (p *Parser) next() lexer.Token {
	t := p.cur()
	if p.idx < len(p.toks)-1 {
		p.idx++
	}
	p.split = 0
	p.lastHi = t.Span.Hi
	return t
}

func (p *Parser) atEOF() bool { return p.toks[p.idx].Kind == lexer.EOF }

// is reports whether the current token is punctuation or a (non-raw)
// keyword/identifier with text s.
func (p *Parser) is(s string) bool { return p.cur().Is(s) }

func (p *Parser) isN(n int, s string) bool { return p.peekN(n).Is(s) }

func (p *Parser) eat(s string) bool {
	if p.is(s) {
		p.next()
		return true
	}
	return false
}

// eatPrefix consumes the punctuation s, splitting a longer compound token
// that starts with s (e.g. `>` from `>>`).
func (p *Parser) eatPrefix(s string) bool {
	t := p.cur()
	if t.Kind != lexer.Punct {
		return false
	}
	if t.Text == s {
		p.next()
		return true
	}
	if strings.HasPrefix(t.Text, s) {
		p.split += len(s)
		hi := t.Span.Lo
		hi.Off += len(s)
		hi.Col += len(s)
		p.lastHi = hi
		return true
	}
	return false
}

// isPrefix reports whether the current punct token starts with s.
func (p *Parser) isPrefix(s string) bool {
	t := p.cur()
	return t.Kind == lexer.Punct && strings.HasPrefix(t.Text, s)
}

func (p *Parser) expect(s string) lexer.Token {
	if !p.is(s) {
		p.errExpected("`" + s + "`")
	}
	return p.next()
}

func (p *Parser) expectPrefix(s string) {
	if !p.eatPrefix(s) {
		p.errExpected("`" + s + "`")
	}
}

func describe(t lexer.Token) string {
	switch t.Kind {
	case lexer.EOF:
		return "end of input"
	case lexer.Str:
		return fmt.Sprintf("string literal %q", t.Text)
	case lexer.ByteStr:
		return fmt.Sprintf("byte string literal b%q", t.Text)
	case lexer.CStr:
		return fmt.Sprintf("c string literal c%q", t.Text)
	case lexer.Char:
		return fmt.Sprintf("char literal %q", t.Text)
	case lexer.Byte:
		return fmt.Sprintf("byte literal b%q", t.Text)
	case lexer.Int, lexer.Float:
		return "literal `" + t.Text + t.Suffix + "`"
	case lexer.Ident:
		if t.Raw {
			return "`r#" + t.Text + "`"
		}
		if t.IsKeyword() {
			return "keyword `" + t.Text + "`"
		}
		return "identifier `" + t.Text + "`"
	}
	return "`" + t.Text + "`"
}

func (p *Parser) errorf(format string, args ...any) {
	panic(parseError{&Error{Pos: p.cur().Span.Lo, Msg: fmt.Sprintf(format, args...)}})
}

func (p *Parser) errExpected(what string) {
	p.errorf("expected %s, found %s", what, describe(p.cur()))
}

func (p *Parser) catch(err *error) {
	if r := recover(); r != nil {
		if pe, ok := r.(parseError); ok {
			*err = pe.err
			return
		}
		panic(r)
	}
}

func (p *Parser) span(lo lexer.Pos) lexer.Span {
	hi := p.lastHi
	if hi.Off < lo.Off {
		hi = lo
	}
	return lexer.Span{Lo: lo, Hi: hi}
}

func (p *Parser) eb(lo lexer.Pos) ast.ExprBase {
	return ast.ExprBase{Span: p.span(lo), ID: ast.NewID()}
}
func (p *Parser) pb(lo lexer.Pos) ast.PatBase { return ast.PatBase{Span: p.span(lo), ID: ast.NewID()} }
func (p *Parser) tb(lo lexer.Pos) ast.TypeBase {
	return ast.TypeBase{Span: p.span(lo), ID: ast.NewID()}
}

func (p *Parser) lo() lexer.Pos { return p.cur().Span.Lo }

// isIdent reports whether the current token is a non-keyword identifier (raw
// identifiers included).
func isPlainIdent(t lexer.Token) bool {
	return t.Kind == lexer.Ident && (t.Raw || !lexer.Keywords[t.Text])
}

func (p *Parser) parseIdentTok() ast.Ident {
	t := p.cur()
	if !isPlainIdent(t) {
		p.errExpected("identifier")
	}
	p.next()
	return ast.Ident{Name: t.Text, Span: t.Span, Hyg: t.Hyg}
}

// ---------------------------------------------------------------------------
// Exported API (used by the macro expander)

// AtEOF reports whether all tokens have been consumed.
func (p *Parser) AtEOF() bool { return p.atEOF() }

// Peek returns the current token without consuming it.
func (p *Parser) Peek() lexer.Token { return p.cur() }

// PeekN returns the token n positions ahead (0 = current).
func (p *Parser) PeekN(n int) lexer.Token { return p.peekN(n) }

// Next consumes and returns the current token.
func (p *Parser) Next() lexer.Token { return p.next() }

// Pos returns an opaque position usable with Reset. (It encodes the token
// index and the number of bytes consumed from a partially split token.)
func (p *Parser) Pos() int { return p.idx<<3 | p.split }

// Reset rewinds (or advances) the parser to a position returned by Pos.
func (p *Parser) Reset(pos int) {
	p.idx = pos >> 3
	p.split = pos & 7
	if p.split > 0 {
		t := p.toks[p.idx]
		p.lastHi = t.Span.Lo
		p.lastHi.Off += p.split
		p.lastHi.Col += p.split
	} else if p.idx > 0 {
		p.lastHi = p.toks[p.idx-1].Span.Hi
	} else {
		p.lastHi = p.toks[0].Span.Lo
	}
}

// TokenIndex returns the index of the current token in the flattened token
// list (ignoring partial splits).
func (p *Parser) TokenIndex() int { return p.idx }

// Tokens returns the flattened token list the parser runs over (ends with EOF).
func (p *Parser) Tokens() []lexer.Token { return p.toks }

// ParseExpr parses an expression.
func (p *Parser) ParseExpr() (e ast.Expr, err error) {
	defer p.catch(&err)
	return p.parseExpr(), nil
}

// ParseType parses a type (bounds with `+` allowed).
func (p *Parser) ParseType() (t ast.Type, err error) {
	defer p.catch(&err)
	return p.parseType(), nil
}

// ParsePat parses a top-level pattern (alternatives with `|` allowed).
func (p *Parser) ParsePat() (pat ast.Pat, err error) {
	defer p.catch(&err)
	return p.parsePat(), nil
}

// ParsePatNoTopAlt parses a pattern without top-level `|` alternatives.
func (p *Parser) ParsePatNoTopAlt() (pat ast.Pat, err error) {
	defer p.catch(&err)
	return p.parsePatNoTopAlt(), nil
}

// ParseItems parses items until EOF.
func (p *Parser) ParseItems() (items []*ast.Item, err error) {
	defer p.catch(&err)
	for !p.atEOF() {
		if p.eat(";") {
			continue
		}
		items = append(items, p.parseItem())
	}
	return items, nil
}

// ParseItem parses a single item (with its outer attributes).
func (p *Parser) ParseItem() (it *ast.Item, err error) {
	defer p.catch(&err)
	return p.parseItem(), nil
}

// ParseAssocItems parses trait/impl associated items until EOF.
func (p *Parser) ParseAssocItems() (items []*ast.Item, err error) {
	defer p.catch(&err)
	for !p.atEOF() {
		if p.eat(";") {
			continue
		}
		items = append(items, p.parseAssocItem())
	}
	return items, nil
}

// ParseStmts parses block-body statements until EOF, using the same rules as
// inside a block (the final expression without `;` has Semi == false).
func (p *Parser) ParseStmts() (stmts []ast.Stmt, err error) {
	defer p.catch(&err)
	return p.parseStmtsUntil(true), nil
}

// ParseStmt parses a single statement (as for the `stmt` macro fragment).
// A trailing semicolon is consumed only if present.
func (p *Parser) ParseStmt() (s ast.Stmt, err error) {
	defer p.catch(&err)
	return p.parseStmt(), nil
}

// ParsePath parses an expression-style path (generic args need `::<`).
func (p *Parser) ParsePath() (path *ast.Path, err error) {
	defer p.catch(&err)
	return p.parsePath(styleExpr), nil
}

// ParseTypePath parses a type-style path (`Vec<T>`, `Fn(A) -> B`).
func (p *Parser) ParseTypePath() (path *ast.Path, err error) {
	defer p.catch(&err)
	return p.parsePath(styleType), nil
}

// ParseIdent parses any identifier token (keywords allowed, `_` not), as
// for the `ident` macro fragment.
func (p *Parser) ParseIdent() (id ast.Ident, err error) {
	defer p.catch(&err)
	t := p.cur()
	if t.Kind != lexer.Ident || (t.Text == "_" && !t.Raw) {
		p.errExpected("identifier")
	}
	p.next()
	return ast.Ident{Name: t.Text, Span: t.Span, Hyg: t.Hyg}, nil
}

// ParseVisibility parses an optional visibility qualifier.
func (p *Parser) ParseVisibility() (v ast.Visibility, err error) {
	defer p.catch(&err)
	return p.parseVis(), nil
}

// ParseLifetime parses a lifetime token and returns its text (e.g. "'a").
func (p *Parser) ParseLifetime() (lt string, err error) {
	defer p.catch(&err)
	t := p.cur()
	if t.Kind != lexer.Lifetime {
		p.errExpected("lifetime")
	}
	p.next()
	return t.Text, nil
}

// ParseLiteral parses a literal, optionally preceded by `-` (as for the
// `literal` macro fragment). The result is a *LitExpr or a UnaryExpr(Neg).
func (p *Parser) ParseLiteral() (e ast.Expr, err error) {
	defer p.catch(&err)
	lo := p.lo()
	if p.is("-") {
		p.next()
		lit := p.parseLit()
		return &ast.UnaryExpr{ExprBase: p.eb(lo), Op: ast.UnNeg, X: lit}, nil
	}
	return p.parseLit(), nil
}

// ParseBlock parses a `{ ... }` block.
func (p *Parser) ParseBlock() (b *ast.Block, err error) {
	defer p.catch(&err)
	return p.parseBlock(), nil
}

// ParseAttrs parses any outer attributes (`#[...]`) at the current position.
func (p *Parser) ParseAttrs() (attrs []*ast.Attr, err error) {
	defer p.catch(&err)
	return p.parseOuterAttrs(), nil
}

// ParseMeta parses an attribute body (`path`, `path(tokens)`, `path = expr`),
// as for the `meta` macro fragment.
func (p *Parser) ParseMeta() (a *ast.Attr, err error) {
	defer p.catch(&err)
	lo := p.lo()
	path := p.parsePath(styleMod)
	var segs []string
	for _, s := range path.Segments {
		segs = append(segs, s.Ident.Name)
	}
	a = &ast.Attr{Path: strings.Join(segs, "::")}
	if p.is("(") || p.is("[") || p.is("{") {
		a.Tokens = []ast.TokenTree{p.parseDelimGroup()}
	} else if p.is("=") {
		start := p.idx
		p.next()
		p.parseExpr()
		tts, _ := TokenTrees(p.toks[start:p.idx])
		a.Tokens = tts
	}
	a.Span = p.span(lo)
	return a, nil
}

// ParseTokenTree consumes a single token tree (a token or a delimited group).
func (p *Parser) ParseTokenTree() (tt ast.TokenTree, err error) {
	defer p.catch(&err)
	if p.atEOF() {
		p.errExpected("token")
	}
	if p.split == 0 && (p.is("(") || p.is("[") || p.is("{")) {
		return p.parseDelimGroup(), nil
	}
	t := p.cur()
	if t.Kind == lexer.Punct && (t.Text == ")" || t.Text == "]" || t.Text == "}") {
		p.errorf("unexpected closing delimiter `%s`", t.Text)
	}
	return ast.TokenTree{Tok: p.next()}, nil
}

// ---------------------------------------------------------------------------
// Token trees and attributes

// parseDelimGroup consumes a delimited group starting at the current
// opening delimiter.
func (p *Parser) parseDelimGroup() ast.TokenTree {
	open := p.cur()
	if open.Kind != lexer.Punct || (open.Text != "(" && open.Text != "[" && open.Text != "{") {
		p.errExpected("one of `(`, `[`, `{`")
	}
	p.next()
	tt := ast.TokenTree{Delim: open.Text[0], Open: open.Span}
	want := string(closerOf(open.Text[0]))
	for {
		t := p.cur()
		if t.Kind == lexer.EOF {
			panic(parseError{&Error{Pos: open.Span.Lo, Msg: fmt.Sprintf("unclosed delimiter `%s`", open.Text)}})
		}
		if t.Kind == lexer.Punct {
			switch t.Text {
			case ")", "]", "}":
				if t.Text != want {
					p.errorf("mismatched closing delimiter: expected `%s`, found `%s`", want, t.Text)
				}
				p.next()
				tt.Close = t.Span
				return tt
			case "(", "[", "{":
				if p.split == 0 {
					tt.Inner = append(tt.Inner, p.parseDelimGroup())
					continue
				}
			}
		}
		tt.Inner = append(tt.Inner, ast.TokenTree{Tok: p.next()})
	}
}

func (p *Parser) isOuterAttrStart() bool {
	return p.is("#") && p.isN(1, "[")
}

func (p *Parser) isInnerAttrStart() bool {
	return p.is("#") && p.isN(1, "!") && p.isN(2, "[")
}

func (p *Parser) parseOuterAttrs() []*ast.Attr {
	var attrs []*ast.Attr
	for p.isOuterAttrStart() {
		attrs = append(attrs, p.parseAttr(false))
	}
	return attrs
}

func (p *Parser) parseInnerAttrs() []*ast.Attr {
	var attrs []*ast.Attr
	for p.isInnerAttrStart() {
		attrs = append(attrs, p.parseAttr(true))
	}
	return attrs
}

func (p *Parser) parseAttr(inner bool) *ast.Attr {
	lo := p.lo()
	p.expect("#")
	if inner {
		p.expect("!")
	}
	if !p.is("[") {
		p.errExpected("`[`")
	}
	g := p.parseDelimGroup()
	a := &ast.Attr{Inner: inner, Span: p.span(lo)}
	tts := g.Inner
	i := 0
	var segs []string
	if i < len(tts) && tts[i].Delim == 0 && tts[i].Tok.Is("::") {
		i++
	}
	for i < len(tts) && tts[i].Delim == 0 && tts[i].Tok.Kind == lexer.Ident {
		segs = append(segs, tts[i].Tok.Text)
		i++
		if i+1 < len(tts) && tts[i].Delim == 0 && tts[i].Tok.Is("::") && tts[i+1].Delim == 0 && tts[i+1].Tok.Kind == lexer.Ident {
			i++
			continue
		}
		break
	}
	if len(segs) == 0 {
		panic(parseError{&Error{Pos: g.Open.Hi, Msg: "expected attribute path"}})
	}
	a.Path = strings.Join(segs, "::")
	a.Tokens = tts[i:]
	return a
}

// ---------------------------------------------------------------------------
// Visibility

func (p *Parser) parseVis() ast.Visibility {
	if !p.is("pub") {
		return ast.VisPrivate
	}
	p.next()
	if p.is("(") && p.split == 0 {
		t1 := p.peekN(1)
		if (t1.Is("crate") || t1.Is("self") || t1.Is("super")) && p.isN(2, ")") {
			p.next()
			p.next()
			p.next()
			switch t1.Text {
			case "crate":
				return ast.VisCrate
			case "self":
				return ast.VisPrivate
			}
			return ast.VisRestricted
		}
		if t1.Is("in") {
			p.next()
			p.next()
			path := p.parsePath(styleMod)
			p.expect(")")
			if len(path.Segments) == 1 && path.Segments[0].Ident.Name == "crate" {
				return ast.VisCrate
			}
			if len(path.Segments) == 1 && path.Segments[0].Ident.Name == "self" {
				return ast.VisPrivate
			}
			return ast.VisRestricted
		}
	}
	return ast.VisPub
}
