package expand

import (
	"fmt"
	"strconv"
	"strings"

	"rustgo/compiler/ast"
	"rustgo/compiler/lexer"
	"rustgo/compiler/parser"
)

// deriveInfo describes the type a derive is applied to.
type deriveInfo struct {
	name     string
	generics *ast.Generics
	strct    *ast.StructItem
	enum     *ast.EnumItem
	union    *ast.UnionItem
	span     lexer.Span
}

type deriveFn func(d *deriveInfo) (string, error)

var derives = map[string]struct {
	trait string
	gen   deriveFn
}{
	"Debug":      {"::std::fmt::Debug", deriveDebug},
	"Clone":      {"::std::clone::Clone", deriveClone},
	"Copy":       {"::std::marker::Copy", deriveEmpty},
	"PartialEq":  {"::std::cmp::PartialEq", derivePartialEq},
	"Eq":         {"::std::cmp::Eq", deriveEmpty},
	"PartialOrd": {"::std::cmp::PartialOrd", derivePartialOrd},
	"Ord":        {"::std::cmp::Ord", deriveOrd},
	"Hash":       {"::std::hash::Hash", deriveHash},
	"Default":    {"::std::default::Default", deriveDefault},
}

// derivePaths parses the contents of #[derive(...)] into trait names (the
// last path segment of each entry).
func derivePaths(a *ast.Attr) ([]string, []lexer.Span, error) {
	args, ok := attrArgs(a)
	if !ok {
		return nil, nil, errAt(a.Span, "malformed `derive` attribute input")
	}
	var names []string
	var spans []lexer.Span
	for _, part := range splitTop(args, ",") {
		if len(part) == 0 {
			continue
		}
		last := part[len(part)-1]
		if !isIdentTT(last) {
			return nil, nil, errAt(ttSpan(last), "expected path to a derive macro")
		}
		for i, tt := range part {
			if !(isIdentTT(tt) || isPunct(tt, "::")) || (isIdentTT(tt) && i > 0 && !isPunct(part[i-1], "::")) {
				return nil, nil, errAt(ttSpan(tt), "expected path to a derive macro")
			}
		}
		names = append(names, last.Tok.Text)
		spans = append(spans, ttSpan(last))
	}
	return names, spans, nil
}

// deriveItem generates the impls requested by #[derive] attributes on it
// and removes those attributes.
func (e *Expander) deriveItem(it *ast.Item) ([]*ast.Item, error) {
	if !hasAttr(it.Attrs, "derive") {
		return nil, nil
	}
	d := &deriveInfo{name: it.Name.Name, span: it.Span}
	switch k := it.Kind.(type) {
	case *ast.StructItem:
		d.strct, d.generics = k, k.Generics
	case *ast.EnumItem:
		d.enum, d.generics = k, k.Generics
	case *ast.UnionItem:
		d.union, d.generics = k, k.Generics
	default:
		for _, a := range it.Attrs {
			if a.Path == "derive" {
				return nil, errAt(a.Span, "`derive` may only be applied to `struct`s, `enum`s and `union`s")
			}
		}
	}
	var out []*ast.Item
	var rest []*ast.Attr
	for _, a := range it.Attrs {
		if a.Path != "derive" {
			rest = append(rest, a)
			continue
		}
		names, spans, err := derivePaths(a)
		if err != nil {
			return nil, err
		}
		for i, name := range names {
			dv, ok := derives[name]
			if pm := e.ProcMacros[name]; pm != nil && pm.Kind == PMDerive {
				var others []*ast.Attr
				for _, oa := range it.Attrs {
					if oa.Path != "derive" {
						others = append(others, oa)
					}
				}
				items, err := e.procDerive(pm, it, others, 0)
				if err != nil {
					return nil, err
				}
				out = append(out, items...)
				continue
			}
			if !ok {
				return nil, errAt(spans[i], "cannot find derive macro `%s` in this scope", name)
			}
			if d.union != nil && name != "Clone" && name != "Copy" {
				return nil, errAt(spans[i], "this trait cannot be derived for unions")
			}
			body, err := dv.gen(d)
			if err != nil {
				return nil, errAt(spans[i], "%v", err)
			}
			src := implHeader(d, dv.trait) + " {\n" + body + "\n}"
			items, err := parseDerived(src, a.Span)
			if err != nil {
				return nil, err
			}
			out = append(out, items...)
		}
	}
	it.Attrs = rest
	return out, nil
}

func parseDerived(src string, sp lexer.Span) ([]*ast.Item, error) {
	tts, err := tmpl(sp, src)
	if err != nil {
		return nil, err
	}
	var items []*ast.Item
	err = safeParse(func() error {
		var err error
		items, err = parser.NewFromTokenTrees(tts).ParseItems()
		return err
	})
	if err != nil {
		return nil, errAt(sp, "internal error: derived code does not parse: %v\n%s", err, src)
	}
	return items, nil
}

// globalPath builds a path from "::a::b::C".
func globalPath(s string) *ast.Path {
	p := &ast.Path{Global: strings.HasPrefix(s, "::")}
	for _, seg := range strings.Split(strings.TrimPrefix(s, "::"), "::") {
		p.Segments = append(p.Segments, &ast.PathSegment{Ident: ast.Ident{Name: seg}})
	}
	return p
}

// implHeader renders `impl<G + bounds> Trait for Name<args> where ...`.
func implHeader(d *deriveInfo, trait string) string {
	g := d.generics
	var params string
	if g != nil && len(g.Params) > 0 {
		ng := &ast.Generics{Span: g.Span}
		for _, p := range g.Params {
			cp := *p
			cp.Default = nil
			cp.ConstDefault = nil
			cp.Attrs = nil
			if p.Kind == ast.GPType {
				cp.Bounds = append(append([]*ast.Bound{}, p.Bounds...), &ast.Bound{Path: globalPath(trait)})
			}
			ng.Params = append(ng.Params, &cp)
		}
		params = ast.GenericsParamsString(ng)
	}
	s := "impl" + params + " " + trait + " for " + d.name + ast.GenericsArgsString(g)
	if w := ast.WhereString(g); w != "" {
		s += " " + w
	}
	return s
}

func deriveEmpty(d *deriveInfo) (string, error) { return "", nil }

// fieldName renders a field name for use after `.` or in struct
// expressions/patterns.
func fieldName(f *ast.FieldDef, i int) string {
	if f.Name.Name == "" {
		return strconv.Itoa(i)
	}
	n := f.Name.Name
	if lexer.Keywords[n] && n != "self" && n != "Self" && n != "super" && n != "crate" {
		return "r#" + n
	}
	return n
}

// variantPat renders `Self::V`, `Self::V(p0, p1)` or `Self::V { a: p0 }`
// with bindings prefix0, prefix1, ...
func variantPat(v *ast.Variant, prefix string) string {
	s := "Self::" + v.Name.Name
	switch v.Shape {
	case ast.ShapeTuple:
		var ps []string
		for i := range v.Fields {
			ps = append(ps, fmt.Sprintf("%s%d", prefix, i))
		}
		s += "(" + strings.Join(ps, ", ") + ")"
	case ast.ShapeNamed:
		var ps []string
		for i, f := range v.Fields {
			ps = append(ps, fmt.Sprintf("%s: %s%d", fieldName(f, i), prefix, i))
		}
		s += " { " + strings.Join(ps, ", ") + " }"
	}
	return s
}

// construct renders a constructor `path(e0, e1)` / `path { a: e0 }` / `path`.
func construct(path string, shape ast.VariantShape, fields []*ast.FieldDef, val func(i int) string) string {
	switch shape {
	case ast.ShapeTuple:
		var es []string
		for i := range fields {
			es = append(es, val(i))
		}
		return path + "(" + strings.Join(es, ", ") + ")"
	case ast.ShapeNamed:
		var es []string
		for i, f := range fields {
			es = append(es, fieldName(f, i)+": "+val(i))
		}
		return path + " { " + strings.Join(es, ", ") + " }"
	}
	return path
}

func strLit(s string) string { return rustStr(s) }

// ---------------------------------------------------------------------------
// Debug

func debugFields(name string, shape ast.VariantShape, fields []*ast.FieldDef, val func(i int) string) string {
	switch shape {
	case ast.ShapeUnit:
		return "f.write_str(" + strLit(name) + ")"
	case ast.ShapeTuple:
		s := "f.debug_tuple(" + strLit(name) + ")"
		for i := range fields {
			s += ".field(" + val(i) + ")"
		}
		return s + ".finish()"
	}
	s := "f.debug_struct(" + strLit(name) + ")"
	for i, fd := range fields {
		s += ".field(" + strLit(fd.Name.Name) + ", " + val(i) + ")"
	}
	return s + ".finish()"
}

func deriveDebug(d *deriveInfo) (string, error) {
	var body string
	if d.strct != nil {
		st := d.strct
		body = debugFields(d.name, st.Shape, st.Fields, func(i int) string { return "&self." + fieldName(st.Fields[i], i) })
	} else {
		body = enumMatch(d.enum, "", func(v *ast.Variant) string {
			return debugFields(v.Name.Name, v.Shape, v.Fields, func(i int) string { return fmt.Sprintf("__self_%d", i) })
		})
	}
	return "fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result { " + body + " }", nil
}

// enumMatch renders `match self { Self::V(..) => arm(v), ... }` (or
// `match *self {}` for empty enums). fallback, if non-empty, is added as a
// final `_ => fallback` arm.
func enumMatch(en *ast.EnumItem, fallback string, arm func(v *ast.Variant) string) string {
	if len(en.Variants) == 0 {
		return "match *self {}"
	}
	var b strings.Builder
	b.WriteString("match self { ")
	for _, v := range en.Variants {
		b.WriteString(variantPat(v, "__self_") + " => " + arm(v) + ", ")
	}
	if fallback != "" {
		b.WriteString("_ => " + fallback + ", ")
	}
	b.WriteString("}")
	return b.String()
}

// ---------------------------------------------------------------------------
// Clone

func deriveClone(d *deriveInfo) (string, error) {
	var body string
	switch {
	case d.union != nil:
		body = "*self"
	case d.strct != nil:
		st := d.strct
		body = construct(d.name, st.Shape, st.Fields, func(i int) string {
			return "::std::clone::Clone::clone(&self." + fieldName(st.Fields[i], i) + ")"
		})
	default:
		body = enumMatch(d.enum, "", func(v *ast.Variant) string {
			return construct("Self::"+v.Name.Name, v.Shape, v.Fields, func(i int) string {
				return fmt.Sprintf("::std::clone::Clone::clone(__self_%d)", i)
			})
		})
	}
	return "fn clone(&self) -> Self { " + body + " }", nil
}

// ---------------------------------------------------------------------------
// PartialEq

const discrSelf = "let __self_discr = ::std::intrinsics::discriminant_value(self); "
const discrOther = "let __arg1_discr = ::std::intrinsics::discriminant_value(other); "

// pairMatch renders `match (self, other) { (Self::V(..), Self::V(..)) => arm(v), ..., _ => fallback }`
// over the variants that have fields.
func pairMatch(en *ast.EnumItem, fallback string, arm func(v *ast.Variant) string) (string, bool) {
	var b strings.Builder
	b.WriteString("match (self, other) { ")
	any := false
	for _, v := range en.Variants {
		if len(v.Fields) == 0 {
			continue
		}
		any = true
		b.WriteString("(" + variantPat(v, "__self_") + ", " + variantPat(v, "__arg1_") + ") => " + arm(v) + ", ")
	}
	b.WriteString("_ => " + fallback + ", }")
	return b.String(), any
}

func derivePartialEq(d *deriveInfo) (string, error) {
	var body string
	if d.strct != nil {
		var parts []string
		for i, f := range d.strct.Fields {
			n := fieldName(f, i)
			parts = append(parts, "self."+n+" == other."+n)
		}
		body = "true"
		if len(parts) > 0 {
			body = strings.Join(parts, " && ")
		}
	} else if len(d.enum.Variants) == 0 {
		body = "match *self {}"
	} else {
		m, any := pairMatch(d.enum, "true", func(v *ast.Variant) string {
			var parts []string
			for i := range v.Fields {
				parts = append(parts, fmt.Sprintf("*__self_%d == *__arg1_%d", i, i))
			}
			return strings.Join(parts, " && ")
		})
		body = discrSelf + discrOther + "__self_discr == __arg1_discr"
		if any {
			body += " && " + m
		}
	}
	return "fn eq(&self, other: &Self) -> bool { " + body + " }", nil
}

// ---------------------------------------------------------------------------
// PartialOrd / Ord

// cmpChain renders the lexicographic comparison over n field pairs.
func cmpChain(fn, equalPat, equal string, n int, pair func(i int) (string, string)) string {
	s := equal
	for i := n - 1; i >= 0; i-- {
		a, b := pair(i)
		s = "match " + fn + "(" + a + ", " + b + ") { " + equalPat + " => " + s + ", cmp => cmp, }"
	}
	return s
}

func deriveCmp(d *deriveInfo, fn, equalPat, equal, header string) (string, error) {
	var body string
	if d.strct != nil {
		st := d.strct
		body = cmpChain(fn, equalPat, equal, len(st.Fields), func(i int) (string, string) {
			n := fieldName(st.Fields[i], i)
			return "&self." + n, "&other." + n
		})
	} else if len(d.enum.Variants) == 0 {
		body = "match *self {}"
	} else {
		m, any := pairMatch(d.enum, equal, func(v *ast.Variant) string {
			return cmpChain(fn, equalPat, equal, len(v.Fields), func(i int) (string, string) {
				return fmt.Sprintf("__self_%d", i), fmt.Sprintf("__arg1_%d", i)
			})
		})
		body = discrSelf + discrOther
		if any {
			body += "if __self_discr != __arg1_discr { return " + fn + "(&__self_discr, &__arg1_discr); } " + m
		} else {
			body += fn + "(&__self_discr, &__arg1_discr)"
		}
	}
	return header + " { " + body + " }", nil
}

func derivePartialOrd(d *deriveInfo) (string, error) {
	return deriveCmp(d, "::std::cmp::PartialOrd::partial_cmp",
		"::std::option::Option::Some(::std::cmp::Ordering::Equal)",
		"::std::option::Option::Some(::std::cmp::Ordering::Equal)",
		"fn partial_cmp(&self, other: &Self) -> ::std::option::Option<::std::cmp::Ordering>")
}

func deriveOrd(d *deriveInfo) (string, error) {
	return deriveCmp(d, "::std::cmp::Ord::cmp", "::std::cmp::Ordering::Equal", "::std::cmp::Ordering::Equal",
		"fn cmp(&self, other: &Self) -> ::std::cmp::Ordering")
}

// ---------------------------------------------------------------------------
// Hash

func deriveHash(d *deriveInfo) (string, error) {
	var b strings.Builder
	if d.strct != nil {
		for i, f := range d.strct.Fields {
			b.WriteString("::std::hash::Hash::hash(&self." + fieldName(f, i) + ", state); ")
		}
	} else if len(d.enum.Variants) == 0 {
		b.WriteString("match *self {}")
	} else {
		b.WriteString("::std::hash::Hash::hash(&::std::intrinsics::discriminant_value(self), state); ")
		var arms []string
		hasUnit := false
		for _, v := range d.enum.Variants {
			if len(v.Fields) == 0 {
				hasUnit = true
				continue
			}
			arm := variantPat(v, "__self_") + " => { "
			for i := range v.Fields {
				arm += fmt.Sprintf("::std::hash::Hash::hash(__self_%d, state); ", i)
			}
			arms = append(arms, arm+"}")
		}
		if len(arms) > 0 {
			if hasUnit {
				arms = append(arms, "_ => {}")
			}
			b.WriteString("match self { " + strings.Join(arms, ", ") + " }")
		}
	}
	return "fn hash<__H: ::std::hash::Hasher>(&self, state: &mut __H) { " + b.String() + " }", nil
}

// ---------------------------------------------------------------------------
// Default

func deriveDefault(d *deriveInfo) (string, error) {
	def := func(int) string { return "::std::default::Default::default()" }
	var body string
	if d.strct != nil {
		body = construct(d.name, d.strct.Shape, d.strct.Fields, def)
	} else {
		var chosen *ast.Variant
		for _, v := range d.enum.Variants {
			if hasAttr(v.Attrs, "default") {
				if chosen != nil {
					return "", fmt.Errorf("multiple declared defaults")
				}
				chosen = v
			}
		}
		if chosen == nil {
			return "", fmt.Errorf("no default declared: make a unit variant default by placing `#[default]` above it")
		}
		if chosen.Shape != ast.ShapeUnit && len(chosen.Fields) > 0 {
			return "", fmt.Errorf("the `#[default]` attribute may only be used on unit enum variants")
		}
		body = construct("Self::"+chosen.Name.Name, chosen.Shape, chosen.Fields, def)
	}
	return "fn default() -> Self { " + body + " }", nil
}
