package expand

import (
	"fmt"

	"rustgo/compiler/ast"
	"rustgo/compiler/lexer"
	"rustgo/compiler/parser"
)

// ---------------------------------------------------------------------------
// Macro definitions

type macroDef struct {
	name  string
	crate string // defining crate ("std" => $crate is ::std)
	// exported reports #[macro_export]: the macro lives at its crate's
	// root and is visible to other crates.
	exported bool
	rules    []*macroRule
	span     lexer.Span
	body     []ast.TokenTree // the definition's body (see EncodeCrateState)
}

type macroRule struct {
	lhs []*matcher
	rhs []*tnode
}

type matcherKind int

const (
	mTok   matcherKind = iota // literal token
	mGroup                    // delimited group
	mVar                      // $name:frag
	mRep                      // $( ... ) sep op
)

type matcher struct {
	kind  matcherKind
	tok   lexer.Token
	delim byte
	sub   []*matcher
	name  string
	frag  string
	sep   *lexer.Token
	op    byte // '*', '+', '?'
	vars  []string
	span  lexer.Span
}

var fragKinds = map[string]bool{
	"expr": true, "expr_2021": true, "ident": true, "ty": true, "pat": true, "pat_param": true,
	"path": true, "tt": true, "block": true, "stmt": true, "item": true, "literal": true,
	"lifetime": true, "vis": true, "meta": true,
}

// parseMacroRules parses the body of `macro_rules! name { ... }`.
func parseMacroRules(name string, body []ast.TokenTree, crate string, sp lexer.Span) (*macroDef, error) {
	def := &macroDef{name: name, crate: crate, span: sp, body: body}
	i := 0
	for i < len(body) {
		if body[i].Delim == 0 {
			return nil, errAt(ttSpan(body[i]), "expected macro matcher, found `%s`", tokenText(body[i].Tok))
		}
		lhsTT := body[i]
		i++
		if i >= len(body) || !isPunct(body[i], "=>") {
			return nil, errAt(ttSpan(lhsTT), "expected `=>` after macro matcher in `%s!`", name)
		}
		i++
		if i >= len(body) || body[i].Delim == 0 {
			return nil, errAt(ttSpan(lhsTT), "expected macro transcriber in `%s!`", name)
		}
		rhsTT := body[i]
		i++
		lhs, err := parseMatchers(lhsTT.Inner)
		if err != nil {
			return nil, err
		}
		rhs, err := parseTranscriber(rhsTT.Inner)
		if err != nil {
			return nil, err
		}
		def.rules = append(def.rules, &macroRule{lhs: lhs, rhs: rhs})
		if i < len(body) {
			if !isPunct(body[i], ";") {
				return nil, errAt(ttSpan(body[i]), "expected `;` between macro rules, found `%s`", tokenText(body[i].Tok))
			}
			i++
		}
	}
	if len(def.rules) == 0 {
		return nil, errAt(sp, "macro `%s` has no rules", name)
	}
	return def, nil
}

func isKleeneOp(tt ast.TokenTree) bool {
	return isPunct(tt, "*") || isPunct(tt, "+") || isPunct(tt, "?")
}

// parseRepSuffix parses the `sep? op` after `$( ... )` starting at tts[i].
func parseRepSuffix(tts []ast.TokenTree, i int, sp lexer.Span) (sep *lexer.Token, op byte, next int, err error) {
	if i < len(tts) && isKleeneOp(tts[i]) {
		return nil, tts[i].Tok.Text[0], i + 1, nil
	}
	if i < len(tts) && tts[i].Delim == 0 && i+1 < len(tts) && (isPunct(tts[i+1], "*") || isPunct(tts[i+1], "+")) {
		t := tts[i].Tok
		return &t, tts[i+1].Tok.Text[0], i + 2, nil
	}
	return nil, 0, 0, errAt(sp, "expected one of: `*`, `+`, or `?` after macro repetition")
}

func parseMatchers(tts []ast.TokenTree) ([]*matcher, error) {
	var out []*matcher
	for i := 0; i < len(tts); i++ {
		tt := tts[i]
		if isPunct(tt, "$") && i+1 < len(tts) {
			next := tts[i+1]
			if next.Delim == '(' {
				sub, err := parseMatchers(next.Inner)
				if err != nil {
					return nil, err
				}
				sep, op, ni, err := parseRepSuffix(tts, i+2, ttSpan(next))
				if err != nil {
					return nil, err
				}
				m := &matcher{kind: mRep, sub: sub, sep: sep, op: op, span: tt.Tok.Span}
				m.vars = matcherVars(sub, nil)
				out = append(out, m)
				i = ni - 1
				continue
			}
			if isIdentTT(next) && !(isKw(next, "crate") && !(i+2 < len(tts) && isPunct(tts[i+2], ":"))) {
				if i+3 < len(tts) && isPunct(tts[i+2], ":") && isIdentTT(tts[i+3]) {
					frag := tts[i+3].Tok.Text
					if !fragKinds[frag] {
						return nil, errAt(ttSpan(tts[i+3]), "invalid fragment specifier `%s`", frag)
					}
					out = append(out, &matcher{kind: mVar, name: next.Tok.Text, frag: frag, span: tt.Tok.Span})
					i += 3
					continue
				}
				return nil, errAt(ttSpan(next), "missing fragment specifier for `$%s`", next.Tok.Text)
			}
		}
		if tt.Delim != 0 {
			sub, err := parseMatchers(tt.Inner)
			if err != nil {
				return nil, err
			}
			out = append(out, &matcher{kind: mGroup, delim: tt.Delim, sub: sub, span: tt.Open})
			continue
		}
		out = append(out, &matcher{kind: mTok, tok: tt.Tok, span: tt.Tok.Span})
	}
	return out, nil
}

func matcherVars(ms []*matcher, acc []string) []string {
	for _, m := range ms {
		switch m.kind {
		case mVar:
			acc = append(acc, m.name)
		case mGroup, mRep:
			acc = matcherVars(m.sub, acc)
		}
	}
	return acc
}

// ---------------------------------------------------------------------------
// Transcriber

type tnodeKind int

const (
	tTok tnodeKind = iota
	tGroup
	tVar
	tRep
	tCrate
)

type tnode struct {
	kind tnodeKind
	tt   ast.TokenTree // tTok: the token; tGroup: the group (Inner unused); tVar/tCrate: the `$` token
	name string
	sub  []*tnode
	sep  *lexer.Token
	op   byte
	vars []string
	span lexer.Span
}

func parseTranscriber(tts []ast.TokenTree) ([]*tnode, error) {
	var out []*tnode
	for i := 0; i < len(tts); i++ {
		tt := tts[i]
		if isPunct(tt, "$") && i+1 < len(tts) {
			next := tts[i+1]
			if next.Delim == '(' {
				sub, err := parseTranscriber(next.Inner)
				if err != nil {
					return nil, err
				}
				sep, op, ni, err := parseRepSuffix(tts, i+2, ttSpan(next))
				if err != nil {
					return nil, err
				}
				n := &tnode{kind: tRep, sub: sub, sep: sep, op: op, span: tt.Tok.Span}
				n.vars = tnodeVars(sub, nil)
				out = append(out, n)
				i = ni - 1
				continue
			}
			if isKw(next, "crate") {
				out = append(out, &tnode{kind: tCrate, tt: tt, span: tt.Tok.Span})
				i++
				continue
			}
			if isIdentTT(next) {
				out = append(out, &tnode{kind: tVar, tt: tt, name: next.Tok.Text, span: tt.Tok.Span})
				i++
				continue
			}
		}
		if tt.Delim != 0 {
			sub, err := parseTranscriber(tt.Inner)
			if err != nil {
				return nil, err
			}
			g := tt
			g.Inner = nil
			out = append(out, &tnode{kind: tGroup, tt: g, sub: sub, span: tt.Open})
			continue
		}
		out = append(out, &tnode{kind: tTok, tt: tt, span: tt.Tok.Span})
	}
	return out, nil
}

func tnodeVars(ns []*tnode, acc []string) []string {
	for _, n := range ns {
		switch n.kind {
		case tVar:
			acc = append(acc, n.name)
		case tGroup, tRep:
			acc = tnodeVars(n.sub, acc)
		}
	}
	return acc
}

// ---------------------------------------------------------------------------
// Matching

// mnode is a matched fragment (leaf) or a repetition sequence.
type mnode struct {
	frag *fragment
	seq  []*mnode
}

type fragment struct {
	kind string
	tts  []ast.TokenTree
}

// bindList is an immutable list of bindings (shared between backtracking
// branches).
type bindList struct {
	name string
	val  *mnode
	next *bindList
}

func (b *bindList) lookup(name string) *mnode {
	for ; b != nil; b = b.next {
		if b.name == name {
			return b.val
		}
	}
	return nil
}

type matchState struct {
	steps    int
	exceeded bool
}

const maxMatchSteps = 2_000_000

// matchSeq matches ms[mi:] against in[pos:] and calls k on success. k
// returns true to accept the overall match.
func (st *matchState) matchSeq(ms []*matcher, mi int, in []ast.TokenTree, pos int, b *bindList, k func(int, *bindList) bool) bool {
	st.steps++
	if st.steps > maxMatchSteps {
		st.exceeded = true
		return false
	}
	if mi == len(ms) {
		return k(pos, b)
	}
	m := ms[mi]
	switch m.kind {
	case mTok:
		if pos < len(in) && in[pos].Delim == 0 && tokEq(in[pos].Tok, m.tok) {
			return st.matchSeq(ms, mi+1, in, pos+1, b, k)
		}
		return false
	case mGroup:
		if pos < len(in) && in[pos].Delim == m.delim {
			inner := in[pos].Inner
			return st.matchSeq(m.sub, 0, inner, 0, b, func(p int, b2 *bindList) bool {
				if p != len(inner) {
					return false
				}
				return st.matchSeq(ms, mi+1, in, pos+1, b2, k)
			})
		}
		return false
	case mVar:
		n, ok := matchFragment(m.frag, in, pos)
		if !ok {
			return false
		}
		f := &fragment{kind: m.frag, tts: in[pos : pos+n]}
		return st.matchSeq(ms, mi+1, in, pos+n, &bindList{name: m.name, val: &mnode{frag: f}, next: b}, k)
	case mRep:
		return st.matchRep(m, ms, mi, in, pos, b, nil, k)
	}
	return false
}

func (st *matchState) matchRep(m *matcher, ms []*matcher, mi int, in []ast.TokenTree, pos int, b *bindList, iters []*bindList, k func(int, *bindList) bool) bool {
	canMore := m.op != '?' || len(iters) == 0
	if canMore {
		start := pos
		p := pos
		ok := true
		if len(iters) > 0 && m.sep != nil {
			if p < len(in) && in[p].Delim == 0 && tokEq(in[p].Tok, *m.sep) {
				p++
			} else {
				ok = false
			}
		}
		if ok && p < len(in)+1 {
			if st.matchSeq(m.sub, 0, in, p, nil, func(p2 int, ib *bindList) bool {
				if p2 == start {
					return false // no progress
				}
				next := make([]*bindList, len(iters)+1)
				copy(next, iters)
				next[len(iters)] = ib
				return st.matchRep(m, ms, mi, in, p2, b, next, k)
			}) {
				return true
			}
			if st.exceeded {
				return false
			}
		}
	}
	if m.op == '+' && len(iters) == 0 {
		return false
	}
	// Finish the repetition: bind every variable to the sequence of its
	// per-iteration matches.
	nb := b
	seen := map[string]bool{}
	for _, v := range m.vars {
		if seen[v] {
			continue
		}
		seen[v] = true
		seq := make([]*mnode, len(iters))
		for i, ib := range iters {
			seq[i] = ib.lookup(v)
			if seq[i] == nil {
				seq[i] = &mnode{}
			}
		}
		nb = &bindList{name: v, val: &mnode{seq: seq}, next: nb}
	}
	return st.matchSeq(ms, mi+1, in, pos, nb, k)
}

func tokEq(a, b lexer.Token) bool {
	if a.Kind != b.Kind || a.Text != b.Text {
		return false
	}
	switch a.Kind {
	case lexer.Int, lexer.Float, lexer.Str, lexer.Char:
		return a.Suffix == b.Suffix
	}
	return true
}

// safeParse runs f, converting parser panics into errors.
func safeParse(f func() error) (err error) {
	defer func() {
		if r := recover(); r != nil {
			err = fmt.Errorf("parser panic: %v", r)
		}
	}()
	return f()
}

// parsePrefix runs a parser function on tts and returns how many top-level
// trees it consumed.
func parsePrefix(tts []ast.TokenTree, f func(p *parser.Parser) error) (int, error) {
	var n int
	err := safeParse(func() error {
		p := parser.NewFromTokenTrees(tts)
		if err := f(p); err != nil {
			return err
		}
		var ok bool
		if p.Pos()&7 != 0 {
			return fmt.Errorf("fragment ends inside a token")
		}
		n, ok = treesForFlat(tts, p.TokenIndex())
		if !ok {
			return fmt.Errorf("fragment ends inside a token")
		}
		return nil
	})
	return n, err
}

// matchFragment matches a fragment specifier at in[pos:] and returns the
// number of trees consumed.
func matchFragment(frag string, in []ast.TokenTree, pos int) (int, bool) {
	rest := in[pos:]
	switch frag {
	case "tt":
		return 1, len(rest) > 0
	case "ident":
		if len(rest) > 0 && isIdentTT(rest[0]) && !isUnderscore(rest[0]) {
			return 1, true
		}
		return 0, false
	case "lifetime":
		if len(rest) > 0 && rest[0].Delim == 0 && rest[0].Tok.Kind == lexer.Lifetime {
			return 1, true
		}
		return 0, false
	case "literal":
		i := 0
		if len(rest) > 0 && isPunct(rest[0], "-") {
			i = 1
		}
		if i < len(rest) && rest[i].Delim == 0 && isLiteralTok(rest[i].Tok) {
			if i == 1 && (rest[1].Tok.Kind != lexer.Int && rest[1].Tok.Kind != lexer.Float) {
				return 0, false
			}
			return i + 1, true
		}
		return 0, false
	case "block":
		if len(rest) > 0 && rest[0].Delim == '{' {
			return 1, true
		}
		return 0, false
	case "vis":
		return matchVis(rest), true
	}
	if len(rest) == 0 {
		return 0, false
	}
	var n int
	var err error
	switch frag {
	case "expr", "expr_2021":
		if isUnderscore(rest[0]) {
			return 0, false
		}
		n, err = parsePrefix(rest, func(p *parser.Parser) error { _, err := p.ParseExpr(); return err })
	case "ty":
		n, err = parsePrefix(rest, func(p *parser.Parser) error { _, err := p.ParseType(); return err })
	case "pat":
		n, err = parsePrefix(rest, func(p *parser.Parser) error { _, err := p.ParsePat(); return err })
	case "pat_param":
		n, err = parsePrefix(rest, func(p *parser.Parser) error { _, err := p.ParsePatNoTopAlt(); return err })
	case "path":
		n, err = parsePrefix(rest, func(p *parser.Parser) error { _, err := p.ParseTypePath(); return err })
	case "meta":
		// simple path (no generics: `cfg(..)` is not Fn-sugar), then
		// an optional delimited group or `= expr`
		n = metaPathLen(rest)
		if n == 0 {
			return 0, false
		}
		if n < len(rest) {
			if rest[n].Delim != 0 {
				n++
			} else if isPunct(rest[n], "=") {
				m, err2 := parsePrefix(rest[n+1:], func(p *parser.Parser) error { _, err := p.ParseExpr(); return err })
				if err2 != nil {
					return 0, false
				}
				n += 1 + m
			}
		}
	case "item":
		n, err = parsePrefix(rest, func(p *parser.Parser) error { _, err := p.ParseItem(); return err })
	case "stmt":
		var st ast.Stmt
		n, err = parsePrefix(rest, func(p *parser.Parser) error {
			var err error
			st, err = p.ParseStmt()
			return err
		})
		// The stmt fragment does not include a trailing `;` (except for
		// items such as `struct S;`).
		if err == nil && n > 0 && isPunct(rest[n-1], ";") {
			if _, isItem := st.(*ast.ItemStmt); !isItem {
				n--
			}
		}
	default:
		return 0, false
	}
	if err != nil || n == 0 {
		return 0, false
	}
	return n, true
}

func matchVis(rest []ast.TokenTree) int {
	if len(rest) == 0 || !isKw(rest[0], "pub") {
		return 0
	}
	if len(rest) > 1 && rest[1].Delim == '(' && len(rest[1].Inner) > 0 {
		f := rest[1].Inner[0]
		if isKw(f, "crate") || isKw(f, "self") || isKw(f, "super") || isKw(f, "in") {
			return 2
		}
	}
	return 1
}

// ---------------------------------------------------------------------------
// Invocation and transcription

func (e *Expander) invokeMacroRules(def *macroDef, mac *ast.MacCall) ([]ast.TokenTree, error) {
	st := &matchState{}
	for _, r := range def.rules {
		var binds *bindList
		ok := st.matchSeq(r.lhs, 0, mac.Args, 0, nil, func(p int, b *bindList) bool {
			if p != len(mac.Args) {
				return false
			}
			binds = b
			return true
		})
		if st.exceeded {
			return nil, errAt(mac.Span, "macro `%s!` matching is too complex", def.name)
		}
		if ok {
			hygieneMarks++
			t := &transcriber{def: def, binds: binds, call: mac.Span, mark: hygieneMarks}
			return t.emit(r.rhs, nil)
		}
	}
	return nil, errAt(mac.Span, "no rules of macro `%s` matched this invocation: `%s!(%s)`", def.name, def.name, truncate(TokensString(mac.Args), 80))
}

func truncate(s string, n int) string {
	if len(s) > n {
		return s[:n] + "..."
	}
	return s
}

type transcriber struct {
	def   *macroDef
	binds *bindList
	call  lexer.Span
	mark  uint32 // hygiene mark for identifiers from the macro body
}

// hygieneMarks numbers macro_rules expansions (hygiene of local variables).
var hygieneMarks uint32

func (t *transcriber) lookup(name string, idx []int) (*mnode, bool) {
	n := t.binds.lookup(name)
	if n == nil {
		return nil, false
	}
	for _, i := range idx {
		if n.frag != nil || n.seq == nil {
			break
		}
		if i >= len(n.seq) {
			return nil, false
		}
		n = n.seq[i]
	}
	return n, true
}

func (t *transcriber) emit(ns []*tnode, idx []int) ([]ast.TokenTree, error) {
	var out []ast.TokenTree
	for _, n := range ns {
		switch n.kind {
		case tTok:
			tt := n.tt
			if tt.Delim == 0 && tt.Tok.Kind == lexer.Ident && tt.Tok.Text != "self" && tt.Tok.Text != "Self" {
				// local variables named by the macro body are distinct
				// from the caller's (macro_rules hygiene)
				tt.Tok.Hyg = t.mark
			}
			out = append(out, tt)
		case tGroup:
			inner, err := t.emit(n.sub, idx)
			if err != nil {
				return nil, err
			}
			g := n.tt
			g.Inner = inner
			out = append(out, g)
		case tCrate:
			sp := n.tt.Tok.Span
			if t.def.crate == "main" {
				out = append(out, identTok("crate", sp))
			} else {
				out = append(out, punctTok("::", sp, true), identTok(t.def.crate, sp))
			}
		case tVar:
			m, ok := t.lookup(n.name, idx)
			if !ok {
				// Not a metavariable: emit `$name` literally.
				out = append(out, n.tt, identTok(n.name, n.tt.Tok.Span))
				continue
			}
			if m.frag == nil {
				return nil, errAt(n.span, "variable `%s` is still repeating at this depth", n.name)
			}
			out = append(out, emitFragment(m.frag)...)
		case tRep:
			count := -1
			countVar := ""
			for _, v := range n.vars {
				m, ok := t.lookup(v, idx)
				if !ok || m.frag != nil {
					continue
				}
				if count >= 0 && len(m.seq) != count {
					return nil, errAt(n.span, "meta-variable `%s` repeats %d times, but `%s` repeats %d times", countVar, count, v, len(m.seq))
				}
				count = len(m.seq)
				countVar = v
			}
			if count < 0 {
				return nil, errAt(n.span, "attempted to repeat an expression containing no syntax variables matched as repeating at this depth")
			}
			if n.op == '?' && count > 1 {
				count = 1
			}
			for i := 0; i < count; i++ {
				if i > 0 && n.sep != nil {
					out = append(out, ast.TokenTree{Tok: *n.sep})
				}
				res, err := t.emit(n.sub, append(idx[:len(idx):len(idx)], i))
				if err != nil {
					return nil, err
				}
				out = append(out, res...)
			}
		}
	}
	return out, nil
}

// emitFragment returns the tokens of a captured fragment. Expressions are
// parenthesized to preserve their grouping.
func emitFragment(f *fragment) []ast.TokenTree {
	switch f.kind {
	case "expr", "expr_2021":
		if len(f.tts) > 1 {
			return parenWrap(f.tts)
		}
	case "pat":
		// an or-pattern fragment is one pattern (`c @ $p` with `$p = A | B`
		// is `c @ (A | B)`)
		for _, t := range f.tts {
			if t.Delim == 0 && t.Tok.Text == "|" {
				return parenWrap(f.tts)
			}
		}
	case "path":
		return turbofish(f.tts)
	}
	return f.tts
}

// turbofish rewrites type-style generic arguments in a path (`a::B<T>`) to
// expression style (`a::B::<T>`), which is valid in both type and
// expression position, so that a captured `path` behaves like an opaque
// path nonterminal (e.g. `$p::new()`).
func turbofish(tts []ast.TokenTree) []ast.TokenTree {
	var out []ast.TokenTree
	for i, tt := range tts {
		if i > 0 && (isPunct(tt, "<") || isPunct(tt, "<<")) && isIdentTT(tts[i-1]) {
			sp := tt.Tok.Span
			out = append(out, punctTok("::", lexer.Span{Lo: sp.Lo, Hi: sp.Lo}, true))
		}
		out = append(out, tt)
	}
	return out
}

func macName(mac *ast.MacCall) string {
	if mac.Path == nil || len(mac.Path.Segments) == 0 {
		return ""
	}
	return mac.Path.Segments[len(mac.Path.Segments)-1].Ident.Name
}

// metaPathLen returns the token length of a simple path `a::b::c` (with an
// optional leading `::`) at the start of ts, or 0.
func metaPathLen(ts []ast.TokenTree) int {
	n := 0
	if n < len(ts) && isPunct(ts[n], "::") {
		n++
	}
	for {
		if n >= len(ts) || ts[n].Delim != 0 || ts[n].Tok.Kind != lexer.Ident {
			return 0
		}
		n++
		if n < len(ts) && isPunct(ts[n], "::") {
			n++
			continue
		}
		return n
	}
}
