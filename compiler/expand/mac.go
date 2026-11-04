package expand

import (
	"strings"

	"rustgo/compiler/ast"
	"rustgo/compiler/lexer"
	"rustgo/compiler/parser"
)

// lookupMacroCall resolves the macro invoked by mac. A path-qualified call
// (`$crate::m!`, `krate::m!`, `crate::m!`, `self::m!`) names the crate the
// macro lives in: macro_export'ed macros live at their crate's root, so the
// macro is looked up in that crate rather than by bare name (where another
// crate's textual-scope macro of the same name could shadow it).
func (e *Expander) lookupMacroCall(mac *ast.MacCall) (*macroDef, builtinFn) {
	name := macName(mac)
	if c, ok := e.macroPathCrate(mac.Path); ok {
		if d, b := e.lookupMacroIn(c, name); d != nil || b != nil {
			return d, b
		}
	}
	return e.lookupMacro(name)
}

// macroPathCrate returns the crate (as named in ExpandCrate) a multi-segment
// macro path refers to.
func (e *Expander) macroPathCrate(p *ast.Path) (string, bool) {
	if p == nil || len(p.Segments) < 2 {
		return "", false
	}
	first := p.Segments[0].Ident.Name
	if !p.Global {
		switch first {
		case "crate", "self", "super":
			return e.crate, true
		}
	}
	if k, ok := e.Externs[first]; ok {
		return k, true
	}
	if first == "std" || first == "core" || first == "alloc" {
		return "std", true
	}
	if _, ok := e.macros[first]; ok {
		return first, true
	}
	if !p.Global {
		// a path through a local module
		return e.crate, true
	}
	return "", false
}

// lookupMacroIn looks up a macro defined in (or re-exported under a renaming
// import by) crate c.
func (e *Expander) lookupMacroIn(c, name string) (*macroDef, builtinFn) {
	for i := 0; i < 16; i++ {
		if d, ok := e.macros[c][name]; ok {
			return d, nil
		}
		if c == "std" {
			if b, ok := builtins[name]; ok {
				return nil, b
			}
		}
		orig, ok := e.aliases[c][name]
		if !ok || orig == name {
			break
		}
		name = orig
	}
	return nil, nil
}

// lookupMacro resolves a macro by name: macro_rules! of the current crate,
// then builtins, then macro_rules! of previously expanded crates (std), then
// names a macro was imported under (`use crate::m as n;`).
func (e *Expander) lookupMacro(name string) (*macroDef, builtinFn) {
	if d, b := e.lookupMacroName(name); d != nil || b != nil {
		return d, b
	}
	// follow (possibly chained) renames
	for i := 0; i < 16; i++ {
		orig, ok := e.aliases[e.crate][name]
		if !ok || orig == name {
			break
		}
		if d, b := e.lookupMacroName(orig); d != nil || b != nil {
			return d, b
		}
		name = orig
	}
	return nil, nil
}

func (e *Expander) lookupMacroName(name string) (*macroDef, builtinFn) {
	if d, ok := e.macros[e.crate][name]; ok {
		return d, nil
	}
	// a macro imported from another crate shadows the prelude's
	if imp, ok := e.macroImports[e.crate][strings.Join(e.modPath[1:], "::")+"::"+name]; ok {
		if d, ok := e.macros[imp.crate][imp.name]; ok {
			return d, nil
		}
	}
	if b, ok := builtins[name]; ok {
		return nil, b
	}
	// Other crates: prefer #[macro_export]ed macros (std's are all
	// visible) over another crate's private textual-scope macro.
	var private *macroDef
	for i := len(e.crateOrder) - 1; i >= 0; i-- {
		c := e.crateOrder[i]
		if c == e.crate {
			continue
		}
		if d, ok := e.macros[c][name]; ok {
			if d.exported || c == "std" {
				return d, nil
			}
			if private == nil {
				private = d
			}
		}
	}
	return private, nil
}

// IsCrateMacro reports whether crate (as named in ExpandCrate) defines a
// macro_rules! macro called name, or imports a macro under that name.
func (e *Expander) IsCrateMacro(crate, name string) bool {
	for i := 0; i < 16; i++ {
		if _, ok := e.macros[crate][name]; ok {
			return true
		}
		orig, ok := e.aliases[crate][name]
		if !ok || orig == name {
			return false
		}
		name = orig
		if _, ok := builtins[name]; ok {
			return true
		}
		for _, c := range e.crateOrder {
			if _, ok := e.macros[c][name]; ok {
				return true
			}
		}
	}
	return false
}

// macTokens expands a macro call to tokens. user reports whether a
// macro_rules! macro was invoked (as opposed to a builtin).
func (e *Expander) macTokens(mac *ast.MacCall) (tts []ast.TokenTree, user bool, err error) {
	name := macName(mac)
	def, bi := e.lookupMacroCall(mac)
	if pm := e.procMacroOverride(mac, name); pm != nil {
		// a proc macro named like a builtin (`rustversion::cfg!`)
		def, bi = nil, nil
	}
	switch {
	case def != nil:
		if err := e.checkTextualScope(def, mac); err != nil {
			return nil, true, err
		}
		tts, err = e.invokeMacroRules(def, mac)
		return tts, true, err
	case bi != nil:
		tts, err = bi(e, mac)
		return tts, false, err
	}
	if pm := e.ProcMacros[name]; pm != nil && pm.Kind == PMBang {
		tts, err = pm.Server.Call(PMBang, pm.Fn, nil, mac.Args, mac.Span, e.compileEnv())
		if err != nil {
			return nil, false, errAt(mac.Span, "in `%s!`: %v", name, err)
		}
		return tts, true, nil
	}
	return nil, false, errAt(mac.Span, "cannot find macro `%s` in this scope", name)
}

// checkTextualScope rejects a call of a macro_rules! macro of the crate
// written before every definition of the macro (macro_rules! macros are
// textually scoped: rustc reports "cannot find macro"). Only the clear
// case is rejected: a plain `m!` call in the source (not produced by an
// expansion) of a macro whose definitions are all written later in the
// same file, none exported and none produced by an expansion, with no
// macro of that name elsewhere (std, other crates, imports).
func (e *Expander) checkTextualScope(def *macroDef, mac *ast.MacCall) error {
	if e.site != nil || def.crate != e.crate || e.crate == "std" || mac.Path == nil || mac.Path.Global || len(mac.Path.Segments) != 1 || !realSpan(mac.Span) {
		return nil
	}
	name := def.name
	if macName(mac) != name {
		return nil
	}
	sites := e.defSites[name]
	if len(sites) == 0 {
		return nil
	}
	call := mac.Span.Lo
	for _, s := range sites {
		if s.exported || s.expanded || s.span.Lo.File != call.File || s.span.Lo.Off <= call.Off {
			return nil
		}
	}
	if _, ok := builtins[name]; ok {
		return nil
	}
	if e.usedNames[name] {
		return nil // imported with `use`: path-scoped
	}
	if _, ok := e.aliases[e.crate][name]; ok {
		return nil
	}
	for k := range e.macroImports[e.crate] {
		if k == name || strings.HasSuffix(k, "::"+name) {
			return nil
		}
	}
	for _, c := range e.crateOrder {
		if c == e.crate {
			continue
		}
		if _, ok := e.macros[c][name]; ok {
			return nil
		}
	}
	if e.ProcMacros[name] != nil {
		return nil
	}
	return errAt(mac.Span, "cannot find macro `%s` in this scope", name)
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
	if pos.File < 0 || int(pos.File) >= len(lexer.Files) || pos.Line == 0 {
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
	if !user && macName(mac) != "cfg_select" {
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
	return stmts, user, wrapExpErr(err, mac)
}

func (e *Expander) expandItemMac(mac *ast.MacCall) ([]*ast.Item, bool, error) {
	name := macName(mac)
	def, bi := e.lookupMacroCall(mac)
	var tts []ast.TokenTree
	var user bool
	var err error
	if def == nil && bi != nil {
		switch name {
		case "include", "cfg_select":
			// the file's items are spliced in at this position
			if tts, err = bi(e, mac); err != nil {
				return nil, false, err
			}
		case "compile_error", "thread_local", "include_str", "include_bytes", "env":
			_, err := bi(e, mac)
			return nil, false, err
		default:
			return nil, false, errAt(mac.Span, "macro `%s!` cannot be used in item position", name)
		}
	} else {
		tts, user, err = e.macTokens(mac)
		if err != nil {
			return nil, user, err
		}
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

// procMacroOverride returns the bang proc macro a call names explicitly,
// by a path into another crate (`dep::name!`) or through an import
// (`use dep::name;`), when that takes precedence over the builtin or std
// macro of the same name.
func (e *Expander) procMacroOverride(mac *ast.MacCall, name string) *ProcMacro {
	pm := e.ProcMacros[name]
	if pm == nil || pm.Kind != PMBang {
		return nil
	}
	if c, ok := e.macroPathCrate(mac.Path); ok {
		if c == "std" || c == e.crate {
			return nil
		}
		if d, b := e.lookupMacroIn(c, name); d != nil || b != nil {
			return nil
		}
		return pm
	}
	if imp, ok := e.macroImports[e.crate][strings.Join(e.modPath[1:], "::")+"::"+name]; ok && imp.proc {
		return pm
	}
	return nil
}
