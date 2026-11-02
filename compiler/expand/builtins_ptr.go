package expand

import "rustgo/compiler/ast"

// Pointer / pinning builtin macros (std::ptr::addr_of!, std::pin::pin!,
// core::mem::offset_of!).

func init() {
	registerBuiltin("addr_of", func(e *Expander, mac *ast.MacCall) ([]ast.TokenTree, error) {
		return tmpl(mac.Span, "&raw const $0", mac.Args)
	})
	registerBuiltin("addr_of_mut", func(e *Expander, mac *ast.MacCall) ([]ast.TokenTree, error) {
		return tmpl(mac.Span, "&raw mut $0", mac.Args)
	})
	registerBuiltin("pin", func(e *Expander, mac *ast.MacCall) ([]ast.TokenTree, error) {
		return tmpl(mac.Span, "unsafe { ::std::pin::Pin::new_unchecked(&mut { $0 }) }", mac.Args)
	})
	registerBuiltin("offset_of", func(e *Expander, mac *ast.MacCall) ([]ast.TokenTree, error) {
		parts := splitTop(mac.Args, ",")
		if len(parts) != 2 {
			return nil, errAt(mac.Span, "expected `offset_of!(Type, field)`")
		}
		return tmpl(mac.Span, "{ let __u = ::std::mem::MaybeUninit::<$0>::uninit(); let __b = __u.as_ptr(); unsafe { (&raw const (*__b).$1) as *const u8 as usize - __b as *const u8 as usize } }", parts[0], parts[1])
	})
}

// registerBuiltin adds a builtin macro (callable from init functions in
// other files, independent of the initialization order of `builtins`).
func registerBuiltin(name string, f builtinFn) {
	if builtins == nil {
		builtins = map[string]builtinFn{}
	}
	builtins[name] = f
}
