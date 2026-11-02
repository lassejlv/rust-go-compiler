package expand

import (
	"rustgo/compiler/ast"
	"rustgo/compiler/lexer"
	"rustgo/compiler/parser"
)

// lookupMacro resolves a macro by name: macro_rules! of the current crate,
// then builtins, then macro_rules! of previously expanded crates (std).
func (e *Expander) lookupMacro(name string) (*macroDef, builtinFn) {
	if d, ok := e.macros[e.crate][name]; ok {
		return d, nil
	}
	if b, ok := builtins[name]; ok {
		return nil, b
	}
	for i := len(e.crateOrder) - 1; i >= 0; i-- {
		c := e.crateOrder[i]
		if c == e.crate {
			continue
		}
		if d, ok := e.macros[c][name]; ok {
			return d, nil
		}
	}
	return nil, nil
}

// macTokens expands a macro call to tokens. user reports whether a
// macro_rules! macro was invoked (as opposed to a builtin).
func (e *Expander) macTokens(mac *ast.MacCall) (tts []ast.TokenTree, user bool, err error) {
	name := macName(mac)
	def, bi := e.lookupMacro(name)
	switch {
	case def != nil:
		tts, err = e.invokeMacroRules(def, mac)
		return tts, true, err
	case bi != nil:
		tts, err = bi(e, mac)
		return tts, false, err
	}
	return nil, false, errAt(mac.Span, "cannot find macro `%s` in this scope", name)
}

func describeTok(t lexer.Token) string {
	if t.Kind == lexer.EOF {
		return "end of macro expansion"
	}
	return "`" + tokenText(t) + "`"
}

// checkEOF reports trailing tokens left after parsing a macro expansion.
func checkEOF(p *parser.Parser, mac *ast.MacCall, what string) error {
	if !p.AtEOF() {
		t := p.Peek()
		sp := t.Span
		if !realSpan(sp) {
			sp = mac.Span
		}
		return errAt(sp, "macro expansion ignores token %s and any following (the usage of `%s!` is likely invalid in %s context)", describeTok(t), macName(mac), what)
	}
	return nil
}

func parseExprTokens(tts []ast.TokenTree, mac *ast.MacCall) (ast.Expr, error) {
	if len(tts) == 0 {
		return nil, errAt(mac.Span, "macro expansion of `%s!` is empty in expression position", macName(mac))
	}
	var x ast.Expr
	err := safeParse(func() error {
		p := parser.NewFromTokenTrees(tts)
		var err error
		if x, err = p.ParseExpr(); err != nil {
			return err
		}
		return checkEOF(p, mac, "expression")
	})
	if err != nil {
		return nil, wrapExpErr(err, mac)
	}
	return x, nil
}

// wrapExpErr makes sure an error coming from parsing an expansion has a
// location.
func wrapExpErr(err error, mac *ast.MacCall) error {
	if err == nil {
		return nil
	}
	var pos lexer.Pos
	var msg string
	switch pe := err.(type) {
	case *parser.Error:
		pos, msg = pe.Pos, pe.Msg
	case *lexer.Error:
		pos, msg = pe.Pos, pe.Msg
	default:
		return err
	}
	if pos.File < 0 || pos.File >= len(lexer.Files) || pos.Line == 0 {
		pos = mac.Span.Lo
	}
	return errAt(lexer.Span{Lo: pos, Hi: pos}, "%s (in expansion of `%s!`)", msg, macName(mac))
}

func (e *Expander) expandExprMac(mac *ast.MacCall) (ast.Expr, bool, error) {
	tts, user, err := e.macTokens(mac)
	if err != nil {
		return nil, user, err
	}
	x, err := parseExprTokens(tts, mac)
	return x, user, err
}

func (e *Expander) expandStmtMac(mac *ast.MacCall, semi bool) ([]ast.Stmt, bool, error) {
	tts, user, err := e.macTokens(mac)
	if err != nil {
		return nil, user, err
	}
	if !user {
		x, err := parseExprTokens(tts, mac)
		if err != nil {
			return nil, false, err
		}
		return []ast.Stmt{&ast.ExprStmt{Expr: x, Semi: semi}}, false, nil
	}
	var stmts []ast.Stmt
	err = safeParse(func() error {
		p := parser.NewFromTokenTrees(tts)
		var err error
		if stmts, err = p.ParseStmts(); err != nil {
			return err
		}
		return checkEOF(p, mac, "statement")
	})
	return stmts, true, wrapExpErr(err, mac)
}

func (e *Expander) expandItemMac(mac *ast.MacCall) ([]*ast.Item, bool, error) {
	name := macName(mac)
	def, bi := e.lookupMacro(name)
	if def == nil && bi != nil {
		switch name {
		case "compile_error", "thread_local", "include", "include_str", "include_bytes", "env":
			_, err := bi(e, mac)
			return nil, false, err
		}
		return nil, false, errAt(mac.Span, "macro `%s!` cannot be used in item position", name)
	}
	tts, user, err := e.macTokens(mac)
	if err != nil {
		return nil, user, err
	}
	var items []*ast.Item
	err = safeParse(func() error {
		p := parser.NewFromTokenTrees(tts)
		var err error
		if e.assoc {
			items, err = p.ParseAssocItems()
		} else {
			items, err = p.ParseItems()
		}
		if err != nil {
			return err
		}
		return checkEOF(p, mac, "item")
	})
	return items, user, wrapExpErr(err, mac)
}

func (e *Expander) expandPatMac(mac *ast.MacCall) (ast.Pat, bool, error) {
	tts, user, err := e.macTokens(mac)
	if err != nil {
		return nil, user, err
	}
	if len(tts) == 0 {
		return nil, user, errAt(mac.Span, "macro expansion of `%s!` is empty in pattern position", macName(mac))
	}
	var pat ast.Pat
	err = safeParse(func() error {
		p := parser.NewFromTokenTrees(tts)
		var err error
		if pat, err = p.ParsePat(); err != nil {
			return err
		}
		return checkEOF(p, mac, "pattern")
	})
	return pat, user, wrapExpErr(err, mac)
}

func (e *Expander) expandTypeMac(mac *ast.MacCall) (ast.Type, bool, error) {
	tts, user, err := e.macTokens(mac)
	if err != nil {
		return nil, user, err
	}
	if len(tts) == 0 {
		return nil, user, errAt(mac.Span, "macro expansion of `%s!` is empty in type position", macName(mac))
	}
	var t ast.Type
	err = safeParse(func() error {
		p := parser.NewFromTokenTrees(tts)
		var err error
		if t, err = p.ParseType(); err != nil {
			return err
		}
		return checkEOF(p, mac, "type")
	})
	return t, user, wrapExpErr(err, mac)
}
