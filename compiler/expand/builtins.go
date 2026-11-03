package expand

import (
	"fmt"
	"os"
	"path/filepath"
	"strconv"
	"strings"

	"rustgo/compiler/ast"
	"rustgo/compiler/lexer"
	"rustgo/compiler/parser"
)

// builtinFn expands a builtin macro call into tokens (parsed as an
// expression by the caller).
type builtinFn func(e *Expander, mac *ast.MacCall) ([]ast.TokenTree, error)

var builtins map[string]builtinFn

func init() {
	builtins = map[string]builtinFn{
		"format_args": func(e *Expander, m *ast.MacCall) ([]ast.TokenTree, error) {
			return e.formatArgsMac(m, m.Args, "", "", false)
		},
		"format_args_nl": func(e *Expander, m *ast.MacCall) ([]ast.TokenTree, error) {
			return e.formatArgsMac(m, m.Args, "", "\n", false)
		},
		"const_format_args": func(e *Expander, m *ast.MacCall) ([]ast.TokenTree, error) {
			return e.formatArgsMac(m, m.Args, "", "", false)
		},
		"print":           printMac("::std::io::_print", false),
		"println":         printMac("::std::io::_print", true),
		"eprint":          printMac("::std::io::_eprint", false),
		"eprintln":        printMac("::std::io::_eprint", true),
		"format":          (*Expander).formatMac,
		"write":           writeMac(false),
		"writeln":         writeMac(true),
		"panic":           panicMac("explicit panic", ""),
		"unreachable":     panicMac("internal error: entered unreachable code", "internal error: entered unreachable code: "),
		"todo":            panicMac("not yet implemented", "not yet implemented: "),
		"unimplemented":   panicMac("not implemented", "not implemented: "),
		"assert":          (*Expander).assertMac,
		"debug_assert":    (*Expander).assertMac,
		"assert_eq":       assertCmpMac("=="),
		"debug_assert_eq": assertCmpMac("=="),
		"assert_ne":       assertCmpMac("!="),
		"debug_assert_ne": assertCmpMac("!="),
		"vec":             (*Expander).vecMac,
		"matches":         (*Expander).matchesMac,
		"dbg":             (*Expander).dbgMac,
		"line":            (*Expander).lineMac,
		"column":          (*Expander).columnMac,
		"file":            (*Expander).fileMac,
		"module_path":     (*Expander).modulePathMac,
		"stringify":       (*Expander).stringifyMac,
		"concat":          (*Expander).concatMac,
		"cfg":             (*Expander).cfgMac,
		"env":             (*Expander).envMac,
		"option_env":      (*Expander).optionEnvMac,
		"include":         (*Expander).includeMac,
		"include_str":     (*Expander).includeStrMac,
		"include_bytes":   (*Expander).includeBytesMac,
		"concat_idents":   unsupportedMac,
		"compile_error":   (*Expander).compileErrorMac,
	}
}

// builtinAttrMacros are the builtin attribute macros of the std prelude
// (`core::prelude::v1::derive`, ...), handled by the expander directly.
var builtinAttrMacros = map[string]bool{
	"derive": true, "test": true, "bench": true, "test_case": true, "cfg_attr": true,
	"cfg_eval": true, "global_allocator": true, "alloc_error_handler": true,
	"derive_const": true, "define_opaque": true,
}

// IsBuiltinMacro reports whether name is a builtin (bang or attribute)
// macro; imports of these (`use core::prelude::v1::derive;`) are accepted.
func IsBuiltinMacro(name string) bool {
	_, ok := builtins[name]
	// asm! is parsed directly into ast.AsmExpr
	return ok || builtinAttrMacros[name] || name == "asm"
}

// ---------------------------------------------------------------------------
// Argument helpers

type macArg struct {
	name   string // for `name = expr`
	nameSp lexer.Span
	tts    []ast.TokenTree
}

func (a macArg) span(def lexer.Span) lexer.Span { return ttsSpan(a.tts, def) }

// exprPrefix parses one expression at the start of tts and returns the
// number of trees it spans.
func exprPrefix(tts []ast.TokenTree, mac *ast.MacCall) (int, error) {
	n, err := parsePrefix(tts, func(p *parser.Parser) error { _, err := p.ParseExpr(); return err })
	if err != nil {
		return 0, wrapExpErr(err, mac)
	}
	if n == 0 {
		return 0, errAt(ttsSpan(tts, mac.Span), "expected expression")
	}
	return n, nil
}

// splitExprArgs splits comma-separated expressions (a trailing comma is
// allowed). With named, `ident = expr` arguments are recognized.
func splitExprArgs(tts []ast.TokenTree, named bool, mac *ast.MacCall) ([]macArg, error) {
	var out []macArg
	pos := 0
	for pos < len(tts) {
		var a macArg
		if named && pos+1 < len(tts) && isIdentTT(tts[pos]) && !tts[pos].Tok.IsKeyword() && isPunct(tts[pos+1], "=") {
			a.name = tts[pos].Tok.Text
			a.nameSp = tts[pos].Tok.Span
			pos += 2
			if pos >= len(tts) {
				return nil, errAt(a.nameSp, "expected expression after `%s =`", a.name)
			}
		}
		n, err := exprPrefix(tts[pos:], mac)
		if err != nil {
			return nil, err
		}
		a.tts = tts[pos : pos+n]
		out = append(out, a)
		pos += n
		if pos < len(tts) {
			if !isPunct(tts[pos], ",") {
				return nil, errAt(ttSpan(tts[pos]), "expected `,`, found %s", describeTT(tts[pos]))
			}
			pos++
		}
	}
	return out, nil
}

func describeTT(tt ast.TokenTree) string {
	if tt.Delim != 0 {
		return "`" + string(tt.Delim) + "`"
	}
	return describeTok(tt.Tok)
}

// firstExpr splits `expr [, rest...]`.
func firstExpr(tts []ast.TokenTree, mac *ast.MacCall) (x, rest []ast.TokenTree, err error) {
	if len(tts) == 0 {
		return nil, nil, errAt(mac.Span, "macro `%s!` requires an expression argument", macName(mac))
	}
	n, err := exprPrefix(tts, mac)
	if err != nil {
		return nil, nil, err
	}
	x, rest = tts[:n], tts[n:]
	if len(rest) > 0 {
		if !isPunct(rest[0], ",") {
			return nil, nil, errAt(ttSpan(rest[0]), "expected `,`, found %s", describeTT(rest[0]))
		}
		rest = rest[1:]
	}
	return x, rest, nil
}

// litString evaluates a literal argument (or a nested literal-producing
// builtin macro such as concat!/stringify!) to its string value.
func (e *Expander) litString(tts []ast.TokenTree, mac *ast.MacCall) (string, bool, error) {
	if len(tts) == 1 && isInvisibleGroup(tts[0]) {
		return e.litString(tts[0].Inner, mac)
	}
	if len(tts) == 1 && tts[0].Delim == 0 {
		t := tts[0].Tok
		switch t.Kind {
		case lexer.Str, lexer.Char, lexer.Float:
			return t.Text, t.Kind == lexer.Str, nil
		case lexer.Int:
			if v, err := strconv.ParseUint(strings.ReplaceAll(t.Text, "_", ""), 0, 64); err == nil {
				return strconv.FormatUint(v, 10), false, nil
			}
			return t.Text, false, nil
		case lexer.Ident:
			if t.Text == "true" || t.Text == "false" {
				return t.Text, false, nil
			}
		}
	}
	if len(tts) == 2 && isPunct(tts[0], "-") && tts[1].Delim == 0 && (tts[1].Tok.Kind == lexer.Int || tts[1].Tok.Kind == lexer.Float) {
		s, _, err := e.litString(tts[1:], mac)
		return "-" + s, false, err
	}
	// nested macro: name ! (...)
	if len(tts) >= 3 && tts[len(tts)-1].Delim != 0 && isPunct(tts[len(tts)-2], "!") && isIdentTT(tts[len(tts)-3]) {
		name := tts[len(tts)-3].Tok.Text
		switch name {
		case "concat", "stringify", "file", "line", "column", "module_path", "env":
			g := tts[len(tts)-1]
			inner := &ast.MacCall{Path: &ast.Path{Segments: []*ast.PathSegment{{Ident: ast.Ident{Name: name, Span: tts[0].Tok.Span}}}},
				Delim: g.Delim, Args: g.Inner, Span: ttsSpan(tts, mac.Span)}
			res, err := builtins[name](e, inner)
			if err != nil {
				return "", false, err
			}
			return e.litString(res, inner)
		}
	}
	return "", false, errAt(ttsSpan(tts, mac.Span), "expected a literal")
}

// formatArgsMac expands format_args!-style arguments: a format string
// followed by arguments. prefix/suffix are added to the format string. With
// allowEmpty, an empty argument list means the format string "".
func (e *Expander) formatArgsMac(mac *ast.MacCall, tts []ast.TokenTree, prefix, suffix string, allowEmpty bool) ([]ast.TokenTree, error) {
	fa, _, err := e.formatArgsMacDirect(mac, tts, prefix, suffix, allowEmpty, "")
	return fa, err
}

// formatArgsMacDirect is formatArgsMac with a direct consumer of the
// formatted text (see formatInput.direct); isDirect reports whether the
// result is a call of it rather than an `Arguments` value.
func (e *Expander) formatArgsMacDirect(mac *ast.MacCall, tts []ast.TokenTree, prefix, suffix string, allowEmpty bool, direct string) (fa []ast.TokenTree, isDirect bool, err error) {
	args, err := splitExprArgs(tts, true, mac)
	if err != nil {
		return nil, false, err
	}
	in := &formatInput{byName: map[string]int{}, strSp: mac.Span, direct: direct}
	if len(args) == 0 {
		if !allowEmpty {
			return nil, false, errAt(mac.Span, "requires at least a format string argument")
		}
		in.str = prefix + suffix
	} else {
		if args[0].name != "" {
			return nil, false, errAt(args[0].nameSp, "format argument must be a string literal")
		}
		s, isStr, err := e.litString(args[0].tts, mac)
		if err != nil || !isStr {
			return nil, false, errAt(args[0].span(mac.Span), "format argument must be a string literal")
		}
		in.str = prefix + s + suffix
		in.strSp = args[0].span(mac.Span)
		seenNamed := false
		for _, a := range args[1:] {
			fa := &fmtArg{name: a.name, tts: a.tts, span: a.span(mac.Span)}
			if a.name != "" {
				if _, dup := in.byName[a.name]; dup {
					return nil, false, errAt(a.nameSp, "duplicate argument named `%s`", a.name)
				}
				seenNamed = true
				in.byName[a.name] = len(in.args)
				fa.span = a.nameSp
			} else if seenNamed {
				return nil, false, errAt(fa.span, "positional arguments cannot follow named arguments")
			}
			in.args = append(in.args, fa)
		}
	}
	fa, err = buildFormatArgs(mac.Span, in)
	return fa, in.isDirect, err
}

// location returns "file", line, col of the (outermost) call site.
func (e *Expander) location(mac *ast.MacCall) (string, int, int) {
	sp := e.callSite(mac.Span)
	return lexer.FileName(sp.Lo.File), sp.Lo.Line, sp.Lo.Col
}

func (e *Expander) locArgs(mac *ast.MacCall) string {
	f, l, c := e.location(mac)
	return fmt.Sprintf("%s, %d, %d", rustStr(f), l, c)
}

// ---------------------------------------------------------------------------
// print!, format!, write!

func printMac(fn string, nl bool) builtinFn {
	return func(e *Expander, mac *ast.MacCall) ([]ast.TokenTree, error) {
		suffix := ""
		if nl {
			suffix = "\n"
		}
		// the text is formatted and printed out of line (a call per use)
		fa, direct, err := e.formatArgsMacDirect(mac, mac.Args, "", suffix, nl, "::std::io::__"+strings.TrimPrefix(fn, "::std::io::_"))
		if err != nil {
			return nil, err
		}
		if direct {
			return fa, nil
		}
		return tmpl(mac.Span, fn+"($0)", fa)
	}
}

func (e *Expander) formatMac(mac *ast.MacCall) ([]ast.TokenTree, error) {
	fa, direct, err := e.formatArgsMacDirect(mac, mac.Args, "", "", false, "::std::fmt::__format")
	if err != nil {
		return nil, err
	}
	if direct {
		return fa, nil
	}
	return tmpl(mac.Span, "::std::fmt::format($0)", fa)
}

func writeMac(nl bool) builtinFn {
	return func(e *Expander, mac *ast.MacCall) ([]ast.TokenTree, error) {
		dst, rest, err := firstExpr(mac.Args, mac)
		if err != nil {
			return nil, err
		}
		suffix := ""
		if nl {
			suffix = "\n"
		}
		fa, err := e.formatArgsMac(mac, rest, "", suffix, nl)
		if err != nil {
			return nil, err
		}
		return tmpl(mac.Span, "$0.write_fmt($1)", parenWrap(dst), fa)
	}
}

// ---------------------------------------------------------------------------
// panic! and friends

func panicMac(plain, prefix string) builtinFn {
	return func(e *Expander, mac *ast.MacCall) ([]ast.TokenTree, error) {
		args := mac.Args
		if len(args) == 0 || (len(args) == 1 && isPunct(args[0], ",")) {
			return tmpl(mac.Span, "::std::rt::panic_str("+rustStr(plain)+", "+e.locArgs(mac)+")")
		}
		fa, err := e.formatArgsMac(mac, args, prefix, "", false)
		if err != nil {
			return nil, err
		}
		return tmpl(mac.Span, "::std::rt::panic_fmt($0, "+e.locArgs(mac)+")", fa)
	}
}

// ---------------------------------------------------------------------------
// assert!

// exprSource renders an expression's tokens for dbg! (like stringify!).
func exprSource(tts []ast.TokenTree) string {
	return TokensString(tts)
}

// condSource renders an assert! condition the way rustc does (via the
// expression pretty-printer).
func condSource(tts []ast.TokenTree) string {
	if len(tts) == 1 && isInvisibleGroup(tts[0]) {
		tts = tts[0].Inner
	}
	var s string
	err := safeParse(func() error {
		p := parser.NewFromTokenTrees(tts)
		x, err := p.ParseExpr()
		if err != nil {
			return err
		}
		s = ast.ExprString(x)
		return nil
	})
	if err != nil || s == "" {
		return TokensString(tts)
	}
	return s
}

func (e *Expander) assertMac(mac *ast.MacCall) ([]ast.TokenTree, error) {
	cond, rest, err := firstExpr(mac.Args, mac)
	if err != nil {
		return nil, err
	}
	if len(rest) == 0 {
		msg := "assertion failed: " + condSource(cond)
		return tmpl(mac.Span, "if !($0) { ::std::rt::panic_str("+rustStr(msg)+", "+e.locArgs(mac)+") }", cond)
	}
	fa, err := e.formatArgsMac(mac, rest, "", "", false)
	if err != nil {
		return nil, err
	}
	return tmpl(mac.Span, "if !($0) { ::std::rt::panic_fmt($1, "+e.locArgs(mac)+") }", cond, fa)
}

func assertCmpMac(op string) builtinFn {
	return func(e *Expander, mac *ast.MacCall) ([]ast.TokenTree, error) {
		a, rest, err := firstExpr(mac.Args, mac)
		if err != nil {
			return nil, err
		}
		if len(rest) == 0 {
			return nil, errAt(mac.Span, "macro `%s!` requires two expression arguments", macName(mac))
		}
		b, rest, err := firstExpr(rest, mac)
		if err != nil {
			return nil, err
		}
		msg := []ast.TokenTree(nil)
		if len(rest) > 0 {
			fa, err := e.formatArgsMac(mac, rest, "", "", false)
			if err != nil {
				return nil, err
			}
			msg, err = tmpl(mac.Span, "::std::option::Option::Some($0)", fa)
			if err != nil {
				return nil, err
			}
		} else {
			msg, err = tmpl(mac.Span, "::std::option::Option::None")
			if err != nil {
				return nil, err
			}
		}
		cond := "!(*left_val == *right_val)"
		if op == "!=" {
			cond = "*left_val == *right_val"
		}
		src := "match (&($0), &($1)) { (left_val, right_val) => { if " + cond +
			" { ::std::rt::assert_failed(" + rustStr(op) + ", &*left_val, &*right_val, $2, " + e.locArgs(mac) + "); } } }"
		return tmpl(mac.Span, src, a, b, msg)
	}
}

// ---------------------------------------------------------------------------
// vec!, matches!, dbg!

func (e *Expander) vecMac(mac *ast.MacCall) ([]ast.TokenTree, error) {
	if len(mac.Args) == 0 {
		return tmpl(mac.Span, "::std::vec::Vec::new()")
	}
	n, err := exprPrefix(mac.Args, mac)
	if err != nil {
		return nil, err
	}
	if n < len(mac.Args) && isPunct(mac.Args[n], ";") {
		elem := mac.Args[:n]
		cnt := mac.Args[n+1:]
		if len(cnt) == 0 {
			return nil, errAt(mac.Span, "expected expression after `;` in `vec!`")
		}
		m, err := exprPrefix(cnt, mac)
		if err != nil {
			return nil, err
		}
		if m != len(cnt) {
			return nil, errAt(ttSpan(cnt[m]), "unexpected token %s in `vec!`", describeTT(cnt[m]))
		}
		return tmpl(mac.Span, "::std::vec::from_elem($0, $1)", elem, cnt)
	}
	args, err := splitExprArgs(mac.Args, false, mac)
	if err != nil {
		return nil, err
	}
	var b strings.Builder
	b.WriteString("::std::vec::from_array([")
	var tts [][]ast.TokenTree
	for i, a := range args {
		if i > 0 {
			b.WriteString(", ")
		}
		fmt.Fprintf(&b, "$%d", i)
		tts = append(tts, a.tts)
	}
	b.WriteString("])")
	return tmpl(mac.Span, b.String(), tts...)
}

func (e *Expander) matchesMac(mac *ast.MacCall) ([]ast.TokenTree, error) {
	x, rest, err := firstExpr(mac.Args, mac)
	if err != nil {
		return nil, err
	}
	if len(rest) > 0 && isPunct(rest[len(rest)-1], ",") {
		rest = rest[:len(rest)-1]
	}
	if len(rest) == 0 {
		return nil, errAt(mac.Span, "`matches!` requires a pattern")
	}
	pat, guard := rest, []ast.TokenTree(nil)
	for i, tt := range rest {
		if isKw(tt, "if") {
			pat, guard = rest[:i], rest[i+1:]
			break
		}
	}
	if guard != nil {
		return tmpl(mac.Span, "match $0 { $1 if $2 => true, _ => false }", parenWrap(x), pat, guard)
	}
	return tmpl(mac.Span, "match $0 { $1 => true, _ => false }", parenWrap(x), pat)
}

func escapeBraces(s string) string {
	return strings.NewReplacer("{", "{{", "}", "}}").Replace(s)
}

func (e *Expander) dbgMac(mac *ast.MacCall) ([]ast.TokenTree, error) {
	f, l, c := e.location(mac)
	loc := fmt.Sprintf("[%s:%d:%d]", f, l, c)
	args, err := splitExprArgs(mac.Args, false, mac)
	if err != nil {
		return nil, err
	}
	if len(args) == 0 {
		in := &formatInput{str: escapeBraces(loc) + "\n", strSp: mac.Span}
		fa, err := buildFormatArgs(mac.Span, in)
		if err != nil {
			return nil, err
		}
		return tmpl(mac.Span, "::std::io::_eprint($0)", fa)
	}
	one := func(a macArg) ([]ast.TokenTree, error) {
		str := escapeBraces(loc+" "+exprSource(a.tts)) + " = {:#?}\n"
		tmp, _ := tmpl(mac.Span, "&tmp")
		in := &formatInput{str: str, strSp: mac.Span, args: []*fmtArg{{tts: tmp, span: mac.Span}}}
		fa, err := buildFormatArgs(mac.Span, in)
		if err != nil {
			return nil, err
		}
		return tmpl(mac.Span, "match $0 { tmp => { ::std::io::_eprint($1); tmp } }", parenWrap(a.tts), fa)
	}
	if len(args) == 1 {
		return one(args[0])
	}
	var parts [][]ast.TokenTree
	var b strings.Builder
	b.WriteString("(")
	for i, a := range args {
		t, err := one(a)
		if err != nil {
			return nil, err
		}
		parts = append(parts, t)
		fmt.Fprintf(&b, "$%d, ", i)
	}
	b.WriteString(")")
	return tmpl(mac.Span, b.String(), parts...)
}

// ---------------------------------------------------------------------------
// Literal-producing builtins

func noArgs(mac *ast.MacCall) error {
	if len(mac.Args) != 0 {
		return errAt(mac.Span, "`%s!` takes no arguments", macName(mac))
	}
	return nil
}

func litTok(t lexer.Token, mac *ast.MacCall) []ast.TokenTree {
	t.Span = synthSpan(mac.Span)
	return []ast.TokenTree{{Tok: t}}
}

func (e *Expander) lineMac(mac *ast.MacCall) ([]ast.TokenTree, error) {
	if err := noArgs(mac); err != nil {
		return nil, err
	}
	_, l, _ := e.location(mac)
	return litTok(lexer.Token{Kind: lexer.Int, Text: strconv.Itoa(l), Suffix: "u32"}, mac), nil
}

func (e *Expander) columnMac(mac *ast.MacCall) ([]ast.TokenTree, error) {
	if err := noArgs(mac); err != nil {
		return nil, err
	}
	_, _, c := e.location(mac)
	return litTok(lexer.Token{Kind: lexer.Int, Text: strconv.Itoa(c), Suffix: "u32"}, mac), nil
}

func (e *Expander) fileMac(mac *ast.MacCall) ([]ast.TokenTree, error) {
	if err := noArgs(mac); err != nil {
		return nil, err
	}
	f, _, _ := e.location(mac)
	return litTok(lexer.Token{Kind: lexer.Str, Text: f}, mac), nil
}

func (e *Expander) modulePathMac(mac *ast.MacCall) ([]ast.TokenTree, error) {
	if err := noArgs(mac); err != nil {
		return nil, err
	}
	return litTok(lexer.Token{Kind: lexer.Str, Text: strings.Join(e.modPath, "::")}, mac), nil
}

func (e *Expander) stringifyMac(mac *ast.MacCall) ([]ast.TokenTree, error) {
	return litTok(lexer.Token{Kind: lexer.Str, Text: TokensString(mac.Args)}, mac), nil
}

func (e *Expander) concatMac(mac *ast.MacCall) ([]ast.TokenTree, error) {
	var b strings.Builder
	for _, part := range splitTop(mac.Args, ",") {
		if len(part) == 0 {
			continue
		}
		s, _, err := e.litString(part, mac)
		if err != nil {
			return nil, err
		}
		b.WriteString(s)
	}
	return litTok(lexer.Token{Kind: lexer.Str, Text: b.String()}, mac), nil
}

func (e *Expander) cfgMac(mac *ast.MacCall) ([]ast.TokenTree, error) {
	v, err := EvalCfg(mac.Args, mac.Span)
	if err != nil {
		return nil, err
	}
	return litTok(lexer.Token{Kind: lexer.Ident, Text: strconv.FormatBool(v)}, mac), nil
}

func (e *Expander) envName(mac *ast.MacCall) (string, error) {
	parts := splitTop(mac.Args, ",")
	if len(parts) == 0 || len(parts[0]) == 0 {
		return "", errAt(mac.Span, "env! takes 1 or 2 arguments")
	}
	name, isStr, err := e.litString(parts[0], mac)
	if err != nil {
		return "", err
	}
	if !isStr {
		return "", errAt(mac.Span, "expected string literal")
	}
	return name, nil
}

// lookupEnv consults the crate's compile-time environment (Cargo-provided
// variables and build-script `rustc-env`), then the process environment.
func (e *Expander) lookupEnv(name string) (string, bool) {
	if e.Config != nil {
		if v, ok := e.Config.Env[name]; ok {
			return v, true
		}
	}
	return os.LookupEnv(name)
}

func (e *Expander) envMac(mac *ast.MacCall) ([]ast.TokenTree, error) {
	name, err := e.envName(mac)
	if err != nil {
		return nil, err
	}
	if v, ok := e.lookupEnv(name); ok {
		return tmpl(mac.Span, rustStr(v))
	}
	return nil, errAt(mac.Span, "environment variable `%s` not defined at compile time", name)
}

func (e *Expander) optionEnvMac(mac *ast.MacCall) ([]ast.TokenTree, error) {
	name, err := e.envName(mac)
	if err != nil {
		return nil, err
	}
	if v, ok := e.lookupEnv(name); ok {
		return tmpl(mac.Span, "::std::option::Option::Some::<&'static str>("+rustStr(v)+")")
	}
	return tmpl(mac.Span, "::std::option::Option::None::<&'static str>")
}

// includePath resolves an include!-family path relative to the file
// containing the invocation.
func (e *Expander) includePath(mac *ast.MacCall) (string, error) {
	p, isStr, err := e.litString(mac.Args, mac)
	if err != nil {
		return "", err
	}
	if !isStr {
		return "", errAt(mac.Span, "argument must be a string literal")
	}
	if !filepath.IsAbs(p) {
		p = filepath.Join(filepath.Dir(lexer.FilePath(mac.Span.Lo.File)), p)
	}
	return p, nil
}

func (e *Expander) includeStrMac(mac *ast.MacCall) ([]ast.TokenTree, error) {
	p, err := e.includePath(mac)
	if err != nil {
		return nil, err
	}
	b, err := os.ReadFile(p)
	if err != nil {
		return nil, errAt(mac.Span, "couldn't read `%s`: %v", p, err)
	}
	return tmpl(mac.Span, rustStr(string(b)))
}

func (e *Expander) includeBytesMac(mac *ast.MacCall) ([]ast.TokenTree, error) {
	p, err := e.includePath(mac)
	if err != nil {
		return nil, err
	}
	b, err := os.ReadFile(p)
	if err != nil {
		return nil, errAt(mac.Span, "couldn't read `%s`: %v", p, err)
	}
	var sb strings.Builder
	sb.WriteString("&[")
	for i, c := range b {
		if i > 0 {
			sb.WriteString(", ")
		}
		sb.WriteString(strconv.Itoa(int(c)))
		sb.WriteString("u8")
	}
	sb.WriteString("]")
	return tmpl(mac.Span, sb.String())
}

// includeMac includes a file's tokens (as an expression or items).
func (e *Expander) includeMac(mac *ast.MacCall) ([]ast.TokenTree, error) {
	p, err := e.includePath(mac)
	if err != nil {
		return nil, err
	}
	b, err := os.ReadFile(p)
	if err != nil {
		return nil, errAt(mac.Span, "couldn't read `%s`: %v", p, err)
	}
	return parser.TokenizeToTrees(lexer.AddFile(p, string(b)), string(b))
}

func (e *Expander) compileErrorMac(mac *ast.MacCall) ([]ast.TokenTree, error) {
	parts := splitTop(mac.Args, ",")
	if len(parts) >= 1 && len(parts[0]) > 0 {
		if s, isStr, err := e.litString(parts[0], mac); err == nil && isStr {
			return nil, errAt(mac.Span, "%s", s)
		}
	}
	return nil, errAt(mac.Span, "compile_error! takes 1 argument")
}

func unsupportedMac(e *Expander, mac *ast.MacCall) ([]ast.TokenTree, error) {
	return nil, errAt(mac.Span, "macro `%s!` is not supported", macName(mac))
}
