package typeck

// Well-formedness of built-in attributes on items of local crates: the
// input forms rustc accepts (E0539, E0565, ill_formed_attribute_input),
// the items they apply to, and conflicting or invalid `repr` hints
// (E0587, E0589, E0690, E0731).

import (
	"strconv"

	"rustgo/compiler/ast"
	"rustgo/compiler/lexer"
	. "rustgo/compiler/middle"
)

// attrNameValue returns the value token of `#[name = value]` (ok=false if
// the attribute is not of that form).
func attrNameValue(a *ast.Attr) (lexer.Token, bool) {
	if len(a.Tokens) == 2 && a.Tokens[0].Delim == 0 && a.Tokens[0].Tok.Kind == lexer.Punct && a.Tokens[0].Tok.Text == "=" && a.Tokens[1].Delim == 0 {
		return a.Tokens[1].Tok, true
	}
	return lexer.Token{}, false
}

func attrIsWord(a *ast.Attr) bool { return len(a.Tokens) == 0 }

func attrIsList(a *ast.Attr) bool { return len(a.Tokens) == 1 && a.Tokens[0].Delim == '(' }

func (t *Tcx) checkItemAttrs(d *Def, isFn, isAdt bool, what string) {
	isMain := isFn && d.Name == "main" && d.Parent != nil && d.Parent.Kind == DefCrate
	for _, a := range d.Item.Attrs {
		if a.Inner {
			continue
		}
		switch a.Path {
		case "inline":
			if !isFn {
				continue
			}
			ok := attrIsWord(a)
			if attrIsList(a) {
				in := a.Tokens[0].Inner
				ok = len(in) == 1 && in[0].Delim == 0 && in[0].Tok.Kind == lexer.Ident && (in[0].Tok.Text == "always" || in[0].Tok.Text == "never")
			}
			if !ok {
				t.Errorf(a.Span, "valid forms for the attribute are `inline`, `inline(always)`, and `inline(never)`")
			}
		case "cold", "track_caller", "no_mangle":
			if a.Path == "no_mangle" && what == "constant" && !lintAllowedAt(d, "no_mangle_const_items") {
				t.Errorf(a.Span, "const items should never be `#[no_mangle]`")
			}
			if isFn && !attrIsWord(a) {
				t.ErrorCode(a.Span, "E0565", "malformed `%s` attribute input", a.Path)
			}
			if a.Path == "track_caller" && isMain {
				t.Errorf(a.Span, "`main` function is not allowed to be `#[track_caller]`")
			}
		case "must_use":
			if v, ok := attrNameValue(a); ok && v.Kind != lexer.Str {
				t.ErrorCode(a.Span, "E0539", "malformed `must_use` attribute input")
			} else if !ok && !attrIsWord(a) {
				t.ErrorCode(a.Span, "E0539", "malformed `must_use` attribute input")
			}
		case "export_name", "link_section":
			if !isFn {
				continue
			}
			if v, ok := attrNameValue(a); !ok || v.Kind != lexer.Str {
				t.ErrorCode(a.Span, "E0539", "malformed `%s` attribute input", a.Path)
			}
		case "deprecated":
			bad := false
			if v, ok := attrNameValue(a); ok {
				bad = v.Kind != lexer.Str
			} else if attrIsList(a) {
				in := a.Tokens[0].Inner
				// name = "value" pairs separated by commas
				for i := 0; i < len(in); {
					if in[i].Delim != 0 || in[i].Tok.Kind != lexer.Ident {
						bad = true
						break
					}
					if i+2 < len(in)+0 && in[i+1].Delim == 0 && in[i+1].Tok.Text == "=" {
						if in[i+2].Delim != 0 || in[i+2].Tok.Kind != lexer.Str {
							bad = true
							break
						}
						i += 3
					} else {
						bad = true
						break
					}
					if i < len(in) {
						if in[i].Delim != 0 || in[i].Tok.Text != "," {
							bad = true
							break
						}
						i++
					}
				}
			} else if !attrIsWord(a) {
				bad = true
			}
			if bad {
				t.ErrorCode(a.Span, "E0539", "malformed `deprecated` attribute input")
			}
		case "non_exhaustive":
			if !isAdt || what == "union" {
				t.Errorf(a.Span, "the `non_exhaustive` attribute cannot be used on %ss", what)
			}
		case "test":
			if !isFn {
				t.Errorf(a.Span, "the `test` attribute may only be used on a free function")
			}
		case "repr":
			t.checkReprHints(d, a, what)
		}
	}
}

// checkReprHints reports `repr` hints that conflict or are invalid.
func (t *Tcx) checkReprHints(d *Def, a *ast.Attr, what string) {
	if !attrIsList(a) {
		return
	}
	in := a.Tokens[0].Inner
	packed, align, transparent := false, false, false
	for i, tt := range in {
		if tt.Delim != 0 || tt.Tok.Kind != lexer.Ident {
			continue
		}
		switch tt.Tok.Text {
		case "packed":
			packed = true
			if what == "enum" {
				t.Errorf(tt.Tok.Span, "the `repr(packed)` attribute cannot be used on enums")
			}
		case "transparent":
			transparent = true
		case "align":
			align = true
			if i+1 < len(in) && in[i+1].Delim == '(' && len(in[i+1].Inner) == 1 && in[i+1].Inner[0].Tok.Kind == lexer.Int {
				n, err := strconv.ParseUint(in[i+1].Inner[0].Tok.Text, 0, 64)
				if err == nil && (n == 0 || n&(n-1) != 0) {
					t.ErrorCode(in[i+1].Inner[0].Tok.Span, "E0589", "invalid `repr(align)` attribute: not a power of two")
				}
			}
		}
	}
	if packed && align && what != "enum" {
		t.ErrorCode(a.Span, "E0587", "type has conflicting packed and align representation hints")
	}
	if !transparent || d.Adt == nil {
		return
	}
	switch what {
	case "enum":
		if n := len(d.Adt.Variants); n != 1 {
			t.ErrorCode(a.Span, "E0731", "transparent enum needs exactly one variant, but has %d", n)
		}
	case "struct":
		t.AdtFields(d)
		if len(d.Adt.Variants) != 1 {
			return
		}
		nontrivial := 0
		for _, f := range d.Adt.Variants[0].Fields {
			if f.Ty == nil || f.Ty.HasParam() || f.Ty.HasError() || f.Ty.HasProj() || f.Ty.HasOpaque() {
				return // not judged
			}
			size, al, ok := t.CLayout(f.Ty)
			if !ok {
				return
			}
			if size > 0 || al > 1 {
				nontrivial++
			}
		}
		if nontrivial > 1 {
			t.ErrorCode(a.Span, "E0690", "transparent struct needs at most one non-trivial field, but has %d", nontrivial)
		}
	}
}
