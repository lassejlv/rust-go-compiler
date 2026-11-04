package expand

import (
	"fmt"
	"strconv"
	"strings"
	"unicode/utf8"

	"rustgo/compiler/ast"
	"rustgo/compiler/lexer"
	"rustgo/compiler/parser"
)

// ---------------------------------------------------------------------------
// Errors

// errAt formats an error as `file:line:col: error: message`.
func errAt(sp lexer.Span, format string, args ...any) error {
	return fmt.Errorf("%s: error: %s", sp.Lo, fmt.Sprintf(format, args...))
}

// ---------------------------------------------------------------------------
// Token tree predicates and helpers

func isPunct(tt ast.TokenTree, s string) bool {
	return tt.Delim == 0 && tt.Tok.Kind == lexer.Punct && tt.Tok.Text == s
}

func isIdentTT(tt ast.TokenTree) bool {
	return tt.Delim == 0 && tt.Tok.Kind == lexer.Ident
}

func isKw(tt ast.TokenTree, s string) bool {
	return tt.Delim == 0 && tt.Tok.Kind == lexer.Ident && !tt.Tok.Raw && tt.Tok.Text == s
}

func isUnderscore(tt ast.TokenTree) bool {
	return tt.Delim == 0 && (tt.Tok.Kind == lexer.Ident || tt.Tok.Kind == lexer.Punct) && tt.Tok.Text == "_" && !tt.Tok.Raw
}

func isLiteralTok(t lexer.Token) bool {
	switch t.Kind {
	case lexer.Int, lexer.Float, lexer.Str, lexer.ByteStr, lexer.CStr, lexer.Char, lexer.Byte:
		return true
	case lexer.Ident:
		return !t.Raw && (t.Text == "true" || t.Text == "false")
	}
	return false
}

// ttSpan returns the span covered by a token tree.
func ttSpan(tt ast.TokenTree) lexer.Span {
	if tt.Delim != 0 {
		return lexer.Span{Lo: tt.Open.Lo, Hi: tt.Close.Hi}
	}
	return tt.Tok.Span
}

// ttsSpan returns the span from the first to the last tree (or def if empty).
func ttsSpan(tts []ast.TokenTree, def lexer.Span) lexer.Span {
	if len(tts) == 0 {
		return def
	}
	return lexer.Span{Lo: ttSpan(tts[0]).Lo, Hi: ttSpan(tts[len(tts)-1]).Hi}
}

func closeDelim(d byte) string {
	switch d {
	case '(':
		return ")"
	case '[':
		return "]"
	case '{':
		return "}"
	}
	return ""
}

// flatLen is the number of tokens a tree occupies once flattened (groups
// contribute their open and close delimiters).
func flatLen(tt ast.TokenTree) int {
	if tt.Delim == 0 {
		if tt.Tok.Kind == lexer.DocComment {
			// the parser turns doc comments into #[doc = "..."] / #![doc = ".."]
			if tt.Tok.Inner {
				return 7
			}
			return 6
		}
		return 1
	}
	n := 2
	for _, in := range tt.Inner {
		n += flatLen(in)
	}
	return n
}

// treesForFlat converts a flat token position (as reported by Parser.Pos)
// into a number of top-level trees. ok is false when the position does not
// fall on a tree boundary.
func treesForFlat(tts []ast.TokenTree, flat int) (int, bool) {
	acc := 0
	for i, tt := range tts {
		if acc == flat {
			return i, true
		}
		acc += flatLen(tt)
		if acc > flat {
			return 0, false
		}
	}
	if acc == flat {
		return len(tts), true
	}
	return 0, false
}

// splitTop splits trees at top-level punctuation `sep`.
func splitTop(tts []ast.TokenTree, sep string) [][]ast.TokenTree {
	var out [][]ast.TokenTree
	start := 0
	for i, tt := range tts {
		if isPunct(tt, sep) {
			out = append(out, tts[start:i])
			start = i + 1
		}
	}
	out = append(out, tts[start:])
	return out
}

// group builds a delimited group.
func group(d byte, sp lexer.Span, inner []ast.TokenTree) ast.TokenTree {
	return ast.TokenTree{Delim: d, Open: sp, Close: sp, Inner: inner}
}

// parenWrap wraps trees in parentheses unless they already form one tree.
func parenWrap(tts []ast.TokenTree) []ast.TokenTree {
	if len(tts) == 1 && (tts[0].Delim != 0 || tts[0].Tok.Kind != lexer.Punct) {
		return tts // a lone punctuation token (`..`) still needs parentheses
	}
	sp := ttsSpan(tts, lexer.Span{})
	open := lexer.Span{Lo: sp.Lo, Hi: sp.Lo}
	cl := lexer.Span{Lo: sp.Hi, Hi: sp.Hi}
	return []ast.TokenTree{{Delim: '(', Open: open, Close: cl, Inner: tts}}
}

func identTok(name string, sp lexer.Span) ast.TokenTree {
	return ast.TokenTree{Tok: lexer.Token{Kind: lexer.Ident, Text: name, Span: sp}}
}

func punctTok(p string, sp lexer.Span, joint bool) ast.TokenTree {
	return ast.TokenTree{Tok: lexer.Token{Kind: lexer.Punct, Text: p, Span: sp, JointNext: joint}}
}

func strTok(s string, sp lexer.Span) ast.TokenTree {
	return ast.TokenTree{Tok: lexer.Token{Kind: lexer.Str, Text: s, Span: sp}}
}

// synthSpan is the zero-width span given to tokens synthesized at sp.
func synthSpan(sp lexer.Span) lexer.Span { return lexer.Span{Lo: sp.Lo, Hi: sp.Lo} }

// ---------------------------------------------------------------------------
// Templates: Rust source with $N placeholders replaced by token trees.

// tmpl tokenizes src, gives every template token a zero-width span at sp and
// splices args[N] in place of each `$N`.
func tmpl(sp lexer.Span, src string, args ...[]ast.TokenTree) ([]ast.TokenTree, error) {
	tts, err := parser.TokenizeToTrees(int(sp.Lo.File), src)
	if err != nil {
		return nil, errAt(sp, "internal error: bad macro template %q: %v", src, err)
	}
	return substTemplate(tts, synthSpan(sp), args)
}

func substTemplate(tts []ast.TokenTree, sp lexer.Span, args [][]ast.TokenTree) ([]ast.TokenTree, error) {
	out := make([]ast.TokenTree, 0, len(tts))
	for i := 0; i < len(tts); i++ {
		tt := tts[i]
		if isPunct(tt, "$") && i+1 < len(tts) && tts[i+1].Delim == 0 && tts[i+1].Tok.Kind == lexer.Int {
			n, err := strconv.Atoi(tts[i+1].Tok.Text)
			if err != nil || n >= len(args) {
				return nil, errAt(sp, "internal error: bad template placeholder $%s", tts[i+1].Tok.Text)
			}
			out = append(out, args[n]...)
			i++
			continue
		}
		if tt.Delim != 0 {
			inner, err := substTemplate(tt.Inner, sp, args)
			if err != nil {
				return nil, err
			}
			tt.Inner = inner
			tt.Open, tt.Close = sp, sp
		} else {
			tt.Tok.Span = sp
		}
		out = append(out, tt)
	}
	return out, nil
}

// rustStr renders s as a Rust string literal.
func rustStr(s string) string {
	var b strings.Builder
	b.WriteByte('"')
	for _, r := range s {
		b.WriteString(escapeRune(r, '"'))
	}
	b.WriteByte('"')
	return b.String()
}

// rustChar renders r as a Rust char literal.
func rustChar(r rune) string { return "'" + escapeRune(r, '\'') + "'" }

func escapeRune(r rune, quote rune) string {
	switch r {
	case '\\':
		return `\\`
	case '\n':
		return `\n`
	case '\r':
		return `\r`
	case '\t':
		return `\t`
	case 0:
		return `\0`
	case quote:
		return `\` + string(r)
	}
	if r < 0x20 || r == 0x7f || r == utf8.RuneError {
		return fmt.Sprintf(`\u{%x}`, r)
	}
	return string(r)
}

// ---------------------------------------------------------------------------
// Token printing (stringify!)

// realSpan reports whether a token carries a span pointing at its source text.
func realSpan(sp lexer.Span) bool {
	return sp.Hi.Off > sp.Lo.Off && sp.Lo.File >= 0 && int(sp.Lo.File) < len(lexer.Files) &&
		sp.Hi.File == sp.Lo.File && int(sp.Hi.Off) <= len(lexer.Files[sp.Lo.File].Src)
}

func sourceText(sp lexer.Span) string {
	return lexer.Files[sp.Lo.File].Src[sp.Lo.Off:sp.Hi.Off]
}

// rawStr renders s as a raw string literal (`r"..."`, `r#"..."#` if s
// contains a quote, ...).
func rawStr(s string) string {
	h := ""
	for strings.Contains(s, "\""+h) {
		h += "#"
	}
	return "r" + h + "\"" + s + "\"" + h
}

// tokenText renders one token as Rust source.
func tokenText(t lexer.Token) string {
	if realSpan(t.Span) && t.Kind != lexer.EOF {
		src := sourceText(t.Span)
		if t.Kind == lexer.Str && t.Raw && (strings.HasPrefix(src, "//") || strings.HasPrefix(src, "/*")) {
			// the value of a doc comment's `#[doc = r".."]`
			return rawStr(t.Text)
		}
		// Sanity check: identifiers/puncts must match their text.
		switch t.Kind {
		case lexer.Ident, lexer.Punct, lexer.Lifetime:
			if strings.TrimPrefix(src, "r#") == t.Text {
				return src
			}
		default:
			return src
		}
	}
	switch t.Kind {
	case lexer.Ident:
		if t.Raw {
			return "r#" + t.Text
		}
		return t.Text
	case lexer.Str:
		return rustStr(t.Text) + t.Suffix
	case lexer.ByteStr:
		return "b" + rustStr(t.Text)
	case lexer.CStr:
		return "c" + rustStr(t.Text)
	case lexer.Char:
		r, _ := utf8.DecodeRuneInString(t.Text)
		return rustChar(r) + t.Suffix
	case lexer.Byte:
		r, _ := utf8.DecodeRuneInString(t.Text)
		return "b" + rustChar(r)
	case lexer.Int, lexer.Float:
		return t.Text + t.Suffix
	case lexer.DocComment:
		if t.Inner {
			return "//!" + t.Text + "\n"
		}
		return "///" + t.Text + "\n"
	case lexer.EOF:
		return ""
	}
	return t.Text
}

// TokensString pretty-prints token trees the way rustc's stringify! does
// (rustc_ast_pretty's print_tts): a space separates two trees unless the
// first one's spacing is joint (the next token immediately followed it in
// the source; tokens of a macro_rules body and substituted fragments are
// spaced `Alone`) or rustc's space_between heuristic omits it (`f(x)`,
// `a.b`, `x,`, `#[..]`, ...). A brace group is printed `{ x }` unless its
// first token immediately followed the `{` in the source (`{x}`). Long
// output wraps at rustc's 78-column margin.
func TokensString(tts []ast.TokenTree) string {
	p := newPP()
	printTTs(p, tts)
	return p.eof()
}

// tokensLine is TokensString without line wrapping (for messages).
func tokensLine(tts []ast.TokenTree) string {
	p := newPP()
	p.noWrap = true
	printTTs(p, tts)
	return p.eof()
}

func isPunctTree(tt ast.TokenTree) bool {
	return tt.Delim == 0 && tt.Tok.Kind == lexer.Punct && !isUnderscore(tt)
}

// jointSpacing reports whether tt's spacing is joint (no space follows).
func jointSpacing(tt ast.TokenTree) bool {
	if tt.Delim != 0 {
		return tt.CloseJoint && !isInvisibleGroup(tt)
	}
	return tt.Tok.JointNext
}

// identLike reports an identifier, keyword or `_` token: two of them are
// always separated (rustc never glues `a` `b` into `ab`).
func identLike(tt ast.TokenTree) bool {
	return tt.Delim == 0 && (tt.Tok.Kind == lexer.Ident || isUnderscore(tt))
}

// plainIdent reports an identifier token written as such (not substituted
// for an `ident` fragment), for the space_between rules that look at it.
func plainIdent(tt ast.TokenTree) bool {
	return tt.Delim == 0 && tt.Tok.Kind == lexer.Ident && tt.Tok.Nt&lexer.NtIdent == 0
}

// spaceBetween mirrors rustc_ast_pretty's space_between heuristic. A tree
// ending (a) or starting (b) a substituted fragment stands for rustc's
// invisible-delimited group: neither punctuation nor an identifier.
func spaceBetween(a, b ast.TokenTree) bool {
	aGroup := a.Tok.Nt&lexer.NtEnd != 0 || isInvisibleGroup(a)
	bGroup := b.Tok.Nt&lexer.NtStart != 0 || isInvisibleGroup(b)
	if aGroup {
		// NON-PUNCT + `,`/`;`/`.`
		return bGroup || !(isPunct(b, ",") || isPunct(b, ";") || isPunct(b, "."))
	}
	if a.Delim == 0 && a.Tok.Kind == lexer.DocComment {
		return false
	}
	if isPunct(a, ".") && (bGroup || !isPunctTree(b)) {
		return false
	}
	if bGroup {
		return true
	}
	if isPunct(a, "$") && isIdentTT(b) {
		return false
	}
	if !isPunctTree(a) && (isPunct(b, ",") || isPunct(b, ";") || isPunct(b, ".")) {
		return false
	}
	if plainIdent(a) && isPunct(b, "!") && (a.Tok.Raw || !a.Tok.IsKeyword()) {
		return false // `m!`, but `if !x`
	}
	if plainIdent(a) && b.Delim == '(' {
		if a.Tok.Raw || !a.Tok.IsKeyword() || a.Tok.Text == "fn" || a.Tok.Text == "Self" || a.Tok.Text == "pub" {
			return false
		}
	}
	if isPunct(a, "#") && b.Delim == '[' {
		return false
	}
	return true
}

func printTTs(p *ppPrinter, tts []ast.TokenTree) {
	for i, tt := range tts {
		if tt.Delim == 0 && tt.Tok.Nt&lexer.NtTurbofish != 0 {
			// not part of the source text: drop it (copying)
			kept := append([]ast.TokenTree(nil), tts[:i]...)
			for _, t := range tts[i+1:] {
				if t.Delim != 0 || t.Tok.Nt&lexer.NtTurbofish == 0 {
					kept = append(kept, t)
				}
			}
			tts = kept
			break
		}
	}
	open := 0
	for i, tt := range tts {
		// a substituted fragment is an invisible-delimited group (an
		// `ibox(0)` in rustc's printer)
		if tt.Tok.Nt&lexer.NtStart != 0 {
			p.ibox(0)
			open++
		}
		printTT(p, tt)
		if tt.Tok.Nt&lexer.NtEnd != 0 && open > 0 {
			p.end()
			open--
		}
		if i+1 < len(tts) {
			next := tts[i+1]
			if !jointSpacing(tt) && spaceBetween(tt, next) {
				p.space1()
			} else if identLike(tt) && identLike(next) && tt.Tok.Nt&lexer.NtEnd == 0 && next.Tok.Nt&lexer.NtStart == 0 {
				// rustc never glues two identifiers (`a` `b`)
				p.space1()
			}
		}
	}
	for ; open > 0; open-- {
		p.end()
	}
}

// isInvisibleGroup reports whether tt is a parenthesis group synthesized
// around a captured `expr` fragment (it prints without its parentheses).
func isInvisibleGroup(tt ast.TokenTree) bool {
	if tt.Delim != '(' || len(tt.Inner) == 0 || tt.Open.Lo.Off != tt.Open.Hi.Off {
		return false
	}
	first := ttStart(tt.Inner[0])
	return realSpan(first) && first.Lo.File == tt.Open.Lo.File && first.Lo.Off == tt.Open.Lo.Off
}

func ttStart(tt ast.TokenTree) lexer.Span {
	if tt.Delim != 0 {
		return tt.Open
	}
	return tt.Tok.Span
}

// printTT prints one tree like rustc's print_tt / print_mac_common.
func printTT(p *ppPrinter, tt ast.TokenTree) {
	if isInvisibleGroup(tt) {
		p.ibox(0)
		printTTs(p, tt.Inner)
		p.end()
		return
	}
	if tt.Delim == 0 {
		p.word(tokenText(tt.Tok))
		return
	}
	if tt.Delim != '{' {
		p.word(string(tt.Delim))
		p.ibox(0)
		printTTs(p, tt.Inner)
		p.end()
		p.word(closeDelim(tt.Delim))
		return
	}
	// a brace group's open spacing decides the space after `{` and, for
	// symmetry, before `}`
	space := len(tt.Inner) > 0 && !tt.OpenJoint
	p.cbox(ppIndentUnit)
	p.word("{")
	if space {
		p.space1()
	}
	p.ibox(0)
	printTTs(p, tt.Inner)
	p.end()
	if space {
		p.breakOffset(1, -ppIndentUnit)
	}
	p.word("}")
	p.end()
}
