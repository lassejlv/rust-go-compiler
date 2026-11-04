package expand

import "rustgo/compiler/ast"

// std::task::ready! and core::cfg_select!.

func init() {
	registerBuiltin("ready", func(e *Expander, mac *ast.MacCall) ([]ast.TokenTree, error) {
		args := mac.Args
		if len(args) > 0 && isPunct(args[len(args)-1], ",") {
			args = args[:len(args)-1]
		}
		return tmpl(mac.Span, "match $0 { ::std::task::Poll::Ready(__t) => __t, ::std::task::Poll::Pending => return ::std::task::Poll::Pending, }", parenWrap(args))
	})
	registerBuiltin("cfg_select", (*Expander).cfgSelectMac)
}

// cfgSelectMac expands `cfg_select! { pred => { tokens } ... _ => { tokens } }`
// to the tokens of the first arm whose predicate holds. An arm's body is a
// brace group (its contents are used) or tokens up to the next `,`.
func (e *Expander) cfgSelectMac(mac *ast.MacCall) ([]ast.TokenTree, error) {
	tts := mac.Args
	for i := 0; i < len(tts); {
		j := i
		for j < len(tts) && !isPunct(tts[j], "=>") {
			j++
		}
		if j == len(tts) {
			return nil, errAt(mac.Span, "expected `=>` in `cfg_select!` arm")
		}
		pred := tts[i:j]
		j++
		var body []ast.TokenTree
		if j < len(tts) && tts[j].Delim == '{' {
			body = tts[j].Inner
			j++
			if j < len(tts) && isPunct(tts[j], ",") {
				j++
			}
		} else {
			k := j
			for k < len(tts) && !isPunct(tts[k], ",") {
				k++
			}
			body = tts[j:k]
			j = k
			if j < len(tts) {
				j++
			}
		}
		i = j
		ok := len(pred) == 1 && isUnderscore(pred[0])
		if !ok {
			v, err := EvalCfg(pred, mac.Span)
			if err != nil {
				return nil, err
			}
			ok = v
		}
		if ok {
			return body, nil
		}
	}
	return nil, errAt(mac.Span, "none of the predicates in this `cfg_select` evaluated to true")
}
