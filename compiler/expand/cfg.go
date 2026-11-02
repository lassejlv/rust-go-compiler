package expand

import (
	"strings"

	"rustgo/compiler/ast"
	"rustgo/compiler/lexer"
)

// cfgNames are the set name-only cfg options.
var cfgNames = map[string]bool{
	"debug_assertions": true,
	"unix":             true,
	"test":             false,
	"windows":          false,
	"miri":             false,
	"doc":              false,
	"doctest":          false,
	"panic_unwind":     false,
}

// cfgValues are the set key = "value" cfg options.
var cfgValues = map[string]map[string]bool{
	"target_os":            {"linux": true},
	"target_family":        {"unix": true},
	"target_pointer_width": {"64": true},
	"target_arch":          {"x86_64": true},
	"target_endian":        {"little": true},
	"target_env":           {"gnu": true},
	"target_vendor":        {"unknown": true},
	"panic":                {"abort": true},
	"target_has_atomic":    {"8": true, "16": true, "32": true, "64": true, "ptr": true},
}

// EvalCfg evaluates a cfg predicate given as token trees (the contents of
// `cfg(...)`).
func EvalCfg(tts []ast.TokenTree, sp lexer.Span) (bool, error) {
	parts := splitTop(tts, ",")
	if len(parts) > 0 && len(parts[len(parts)-1]) == 0 {
		parts = parts[:len(parts)-1]
	}
	if len(parts) != 1 {
		return false, errAt(sp, "`cfg` predicate must be a single predicate")
	}
	return evalCfgPred(parts[0], sp)
}

func evalCfgPred(tts []ast.TokenTree, sp lexer.Span) (bool, error) {
	if len(tts) == 0 || !isIdentTT(tts[0]) {
		return false, errAt(ttsSpan(tts, sp), "expected a cfg predicate")
	}
	name := tts[0].Tok.Text
	psp := ttSpan(tts[0])
	switch {
	case len(tts) == 1:
		return cfgNames[name], nil
	case len(tts) == 3 && isPunct(tts[1], "=") && tts[2].Delim == 0 && tts[2].Tok.Kind == lexer.Str:
		if name == "feature" {
			return false, nil
		}
		return cfgValues[name][tts[2].Tok.Text], nil
	case len(tts) == 2 && tts[1].Delim == '(':
		var args [][]ast.TokenTree
		for _, a := range splitTop(tts[1].Inner, ",") {
			if len(a) > 0 {
				args = append(args, a)
			}
		}
		switch name {
		case "not":
			if len(args) != 1 {
				return false, errAt(psp, "expected 1 cfg-pattern")
			}
			v, err := evalCfgPred(args[0], sp)
			return !v, err
		case "all":
			res := true
			for _, a := range args {
				v, err := evalCfgPred(a, sp)
				if err != nil {
					return false, err
				}
				res = res && v
			}
			return res, nil
		case "any":
			res := false
			for _, a := range args {
				v, err := evalCfgPred(a, sp)
				if err != nil {
					return false, err
				}
				res = res || v
			}
			return res, nil
		}
		return false, errAt(psp, "invalid predicate `%s`", name)
	}
	return false, errAt(psp, "malformed cfg predicate")
}

// attrArgs returns the contents of the attribute's parenthesized argument
// list, or false if it has none.
func attrArgs(a *ast.Attr) ([]ast.TokenTree, bool) {
	if len(a.Tokens) == 1 && a.Tokens[0].Delim == '(' {
		return a.Tokens[0].Inner, true
	}
	return nil, false
}

// parseAttrTokens builds an attribute from `path args...` tokens (used by
// cfg_attr).
func parseAttrTokens(tts []ast.TokenTree, inner bool, sp lexer.Span) (*ast.Attr, error) {
	var parts []string
	i := 0
	if i < len(tts) && isPunct(tts[i], "::") {
		i++
	}
	for i < len(tts) && isIdentTT(tts[i]) {
		parts = append(parts, tts[i].Tok.Text)
		i++
		if i < len(tts) && isPunct(tts[i], "::") {
			i++
			continue
		}
		break
	}
	if len(parts) == 0 {
		return nil, errAt(ttsSpan(tts, sp), "expected attribute path")
	}
	return &ast.Attr{Inner: inner, Path: strings.Join(parts, "::"), Tokens: tts[i:], Span: ttsSpan(tts, sp)}, nil
}

// processAttrs expands #[cfg_attr] and evaluates #[cfg]. It returns whether
// the annotated node is kept and the remaining attributes (cfg attributes
// that evaluated to true are removed).
func (e *Expander) processAttrs(attrs []*ast.Attr) (bool, []*ast.Attr, error) {
	if len(attrs) == 0 {
		return true, attrs, nil
	}
	needs := false
	for _, a := range attrs {
		if a.Path == "cfg" || a.Path == "cfg_attr" {
			needs = true
			break
		}
	}
	if !needs {
		return true, attrs, nil
	}
	var out []*ast.Attr
	keep := true
	var walk func(as []*ast.Attr) error
	walk = func(as []*ast.Attr) error {
		for _, a := range as {
			switch a.Path {
			case "cfg":
				args, ok := attrArgs(a)
				if !ok {
					return errAt(a.Span, "`cfg` is not followed by parentheses")
				}
				v, err := EvalCfg(args, a.Span)
				if err != nil {
					return err
				}
				if !v {
					keep = false
				}
			case "cfg_attr":
				args, ok := attrArgs(a)
				if !ok {
					return errAt(a.Span, "malformed `cfg_attr` attribute input")
				}
				parts := splitTop(args, ",")
				if len(parts) < 1 || len(parts[0]) == 0 {
					return errAt(a.Span, "malformed `cfg_attr` attribute input")
				}
				v, err := evalCfgPred(parts[0], a.Span)
				if err != nil {
					return err
				}
				if !v {
					continue
				}
				var sub []*ast.Attr
				for _, p := range parts[1:] {
					if len(p) == 0 {
						continue
					}
					na, err := parseAttrTokens(p, a.Inner, a.Span)
					if err != nil {
						return err
					}
					sub = append(sub, na)
				}
				if err := walk(sub); err != nil {
					return err
				}
			default:
				out = append(out, a)
			}
		}
		return nil
	}
	if err := walk(attrs); err != nil {
		return false, nil, err
	}
	return keep, out, nil
}
