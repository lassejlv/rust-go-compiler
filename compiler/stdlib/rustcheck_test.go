package stdlib

// An optional, heavyweight consistency check of the library sources: the
// macro-expanded crate is printed back as Rust and type-checked by the real
// rustc in `#![no_core]` mode (with a small harness supplying what rustc
// needs beyond our lang items). It only runs when RUSTGO_STD_RUSTC_CHECK is
// set and rustc is installed:
//
//	RUSTGO_STD_RUSTC_CHECK=1 go test ./compiler/stdlib -run Rustc -v
//
// Set RUSTGO_STD_RUSTC_OUT=dir to keep the generated source.

import (
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"regexp"
	"strings"
	"testing"

	"rustgo/compiler/ast"
	"rustgo/compiler/expand"
	"rustgo/compiler/lexer"
)

// itemPrinter renders items (the ast package only prints expressions,
// types, patterns and blocks).
type itemPrinter struct {
	b strings.Builder
	// Rewrites applied while printing for the rustc harness.
	harness bool
	// enclosing trait name (for method-level lang items)
	trait string
}

func (p *itemPrinter) w(format string, args ...any) { fmt.Fprintf(&p.b, format, args...) }

func visString(v ast.Visibility) string {
	switch v {
	case ast.VisPub:
		return "pub "
	case ast.VisCrate, ast.VisRestricted, ast.VisSuper:
		return "pub(crate) "
	}
	return ""
}

// langMap maps our lang item names to rustc's ("" = drop the attribute).
var langMap = map[string]string{
	"into_iterator":  "",
	"from":           "",
	"owned_box":      "",
	"Result":         "",
	"RangeInclusive": "",
	"tuple_trait":    "tuple_trait",
}

func attrLang(a *ast.Attr) (string, bool) {
	if a.Path != "lang" || len(a.Tokens) < 2 {
		return "", false
	}
	return strings.Trim(a.Tokens[1].Tok.Text, `"`), true
}

func (p *itemPrinter) attrs(attrs []*ast.Attr, ind string) {
	for _, a := range attrs {
		switch a.Path {
		case "doc", "inline", "track_caller", "must_use", "non_exhaustive", "cold", "rustc_coerce_unsized", "derive", "default":
			continue
		}
		if name, ok := attrLang(a); ok && p.harness {
			if m, found := langMap[name]; found {
				if m == "" {
					continue
				}
				name = m
			}
			p.w("%s#[lang = %q]\n", ind, name)
			continue
		}
		p.w("%s#[%s%s]\n", ind, a.Path, ast.TokenTreesString(a.Tokens))
	}
}

func (p *itemPrinter) generics(g *ast.Generics) string { return ast.GenericsParamsString(g) }

// genericsDef prints generic parameters including defaults (for type and
// trait definitions).
func (p *itemPrinter) genericsDef(g *ast.Generics) string {
	if g == nil || len(g.Params) == 0 {
		return ""
	}
	var parts []string
	for _, gp := range g.Params {
		s := strings.TrimSuffix(strings.TrimPrefix(ast.GenericsParamsString(&ast.Generics{Params: []*ast.GenericParam{gp}}), "<"), ">")
		if gp.Default != nil {
			s += " = " + ast.TypeString(gp.Default)
		}
		if gp.ConstDefault != nil {
			s += " = " + ast.ExprString(gp.ConstDefault)
		}
		parts = append(parts, s)
	}
	return "<" + strings.Join(parts, ", ") + ">"
}

func (p *itemPrinter) where(g *ast.Generics) string {
	if w := ast.WhereString(g); w != "" {
		return " " + w
	}
	return ""
}

func (p *itemPrinter) fields(fs []*ast.FieldDef, shape ast.VariantShape) string {
	var parts []string
	for _, f := range fs {
		s := visString(f.Vis)
		if shape == ast.ShapeNamed {
			s += f.Name.Name + ": "
		}
		parts = append(parts, s+ast.TypeString(f.Ty))
	}
	switch shape {
	case ast.ShapeNamed:
		return " { " + strings.Join(parts, ", ") + " }"
	case ast.ShapeTuple:
		return "(" + strings.Join(parts, ", ") + ")"
	}
	return ""
}

func (p *itemPrinter) useTree(t *ast.UseTree) string {
	prefix := ""
	if t.Prefix != nil {
		var segs []string
		for _, s := range t.Prefix.Segments {
			segs = append(segs, s.Ident.Name)
		}
		prefix = strings.Join(segs, "::")
		if t.Prefix.Global {
			prefix = "::" + prefix
		}
	}
	join := func(rest string) string {
		if prefix == "" || prefix == "::" {
			return prefix + rest
		}
		return prefix + "::" + rest
	}
	switch t.Kind {
	case ast.UseGlob:
		return join("*")
	case ast.UseNested:
		var parts []string
		for _, n := range t.Nested {
			parts = append(parts, p.useTree(n))
		}
		return join("{" + strings.Join(parts, ", ") + "}")
	}
	s := prefix
	if t.Rename != nil {
		s += " as " + t.Rename.Name
	}
	return s
}

func (p *itemPrinter) fnSig(name string, f *ast.FnItem, forceRustCall bool) string {
	var b strings.Builder
	if f.Const {
		b.WriteString("const ")
	}
	if f.Unsafe {
		b.WriteString("unsafe ")
	}
	if forceRustCall {
		b.WriteString(`extern "rust-call" `)
	} else if f.Abi != "" && f.Abi != "rust-intrinsic" {
		fmt.Fprintf(&b, "extern %q ", f.Abi)
	}
	b.WriteString("fn " + name + p.generics(f.Generics) + "(")
	var params []string
	if sp := f.Decl.SelfParam; sp != nil {
		lt := ""
		if sp.Lifetime != "" {
			lt = sp.Lifetime + " "
		}
		switch sp.Kind {
		case ast.SelfValue:
			if sp.Mut {
				params = append(params, "mut self")
			} else {
				params = append(params, "self")
			}
		case ast.SelfRef:
			params = append(params, "&"+lt+"self")
		case ast.SelfRefMut:
			params = append(params, "&"+lt+"mut self")
		case ast.SelfTyped:
			m := ""
			if sp.Mut {
				m = "mut "
			}
			params = append(params, m+"self: "+ast.TypeString(sp.Ty))
		}
	}
	for _, prm := range f.Decl.Params {
		params = append(params, ast.PatString(prm.Pat)+": "+ast.TypeString(prm.Ty))
	}
	b.WriteString(strings.Join(params, ", ") + ")")
	if f.Decl.Output != nil {
		b.WriteString(" -> " + ast.TypeString(f.Decl.Output))
	}
	b.WriteString(p.where(f.Generics))
	return b.String()
}

func (p *itemPrinter) items(items []*ast.Item, ind string, prelude bool) {
	if prelude && p.harness {
		p.w("%s#[allow(unused_imports)]\n%suse crate::prelude::rust_2021::*;\n", ind, ind)
	}
	for _, it := range items {
		p.item(it, ind)
	}
}

func (p *itemPrinter) item(it *ast.Item, ind string) {
	name := it.Name.Name
	vis := visString(it.Vis)
	switch k := it.Kind.(type) {
	case *ast.MacroRulesItem, *ast.MacCallItem:
		return
	case *ast.ExternBlockItem:
		if p.harness && k.Abi == "rust-intrinsic" {
			// Intrinsics become stub functions.
			for _, f := range k.Items {
				fn, ok := f.Kind.(*ast.FnItem)
				if !ok {
					continue
				}
				// discriminant_value is called from derived code without
				// `unsafe` (like rustc's safe intrinsic).
				fn.Unsafe = f.Name.Name != "discriminant_value"
				fn.Const = true
				p.w("%s%s%s { loop {} }\n", ind, visString(f.Vis), p.fnSig(f.Name.Name, fn, false))
			}
			return
		}
		p.attrs(it.Attrs, ind)
		p.w("%sextern %q {\n", ind, k.Abi)
		for _, f := range k.Items {
			if fn, ok := f.Kind.(*ast.FnItem); ok {
				p.w("%s    %s%s;\n", ind, visString(f.Vis), p.fnSig(f.Name.Name, fn, false))
			}
		}
		p.w("%s}\n", ind)
		return
	}
	p.attrs(it.Attrs, ind)
	switch k := it.Kind.(type) {
	case *ast.FnItem:
		rustCall := false
		if p.harness {
			switch {
			case p.trait == "Iterator" && name == "next":
				p.w("%s#[lang = \"next\"]\n", ind)
			case p.trait == "IntoIterator" && name == "into_iter":
				p.w("%s#[lang = \"into_iter\"]\n", ind)
			case p.trait == "Try" && name == "branch":
				p.w("%s#[lang = \"branch\"]\n", ind)
			case p.trait == "Try" && name == "from_output":
				p.w("%s#[lang = \"from_output\"]\n", ind)
			case p.trait == "FromResidual" && name == "from_residual":
				p.w("%s#[lang = \"from_residual\"]\n", ind)
			case p.trait == "<RangeInclusive>" && name == "new":
				p.w("%s#[lang = \"range_inclusive_new\"]\n", ind)
			case (p.trait == "FnOnce" && name == "call_once") || (p.trait == "FnMut" && name == "call_mut") || (p.trait == "Fn" && name == "call"):
				rustCall = true
			}
		}
		p.w("%s%s%s", ind, vis, p.fnSig(name, k, rustCall))
		if k.Body == nil {
			p.w(";\n")
		} else {
			p.w(" %s\n", ast.BlockString(k.Body))
		}
	case *ast.StructItem:
		g := k.Generics
		switch k.Shape {
		case ast.ShapeNamed:
			p.w("%s%sstruct %s%s%s%s\n", ind, vis, name, p.genericsDef(g), p.where(g), p.fields(k.Fields, k.Shape))
		case ast.ShapeTuple:
			p.w("%s%sstruct %s%s%s%s;\n", ind, vis, name, p.genericsDef(g), p.fields(k.Fields, k.Shape), p.where(g))
		default:
			p.w("%s%sstruct %s%s%s;\n", ind, vis, name, p.genericsDef(g), p.where(g))
		}
	case *ast.UnionItem:
		p.w("%s%sunion %s%s%s%s\n", ind, vis, name, p.genericsDef(k.Generics), p.where(k.Generics), p.fields(k.Fields, ast.ShapeNamed))
	case *ast.EnumItem:
		p.w("%s%senum %s%s%s {\n", ind, vis, name, p.genericsDef(k.Generics), p.where(k.Generics))
		for _, v := range k.Variants {
			p.attrs(v.Attrs, ind+"    ")
			if p.harness && name == "ControlFlow" {
				p.w("%s    #[lang = %q]\n", ind, v.Name.Name)
			}
			p.w("%s    %s%s", ind, v.Name.Name, p.fields(v.Fields, v.Shape))
			if v.Disc != nil {
				p.w(" = %s", ast.ExprString(v.Disc))
			}
			p.w(",\n")
		}
		p.w("%s}\n", ind)
	case *ast.TraitItem:
		if p.harness {
			switch name {
			case "Fn", "FnMut", "FnOnce":
				p.w("%s#[rustc_paren_sugar]\n", ind)
			case "Try":
				p.w("%s#[lang = %q]\n", ind, name)
			}
		}
		u := ""
		if k.Unsafe {
			u = "unsafe "
		}
		if k.Auto {
			u += "auto "
		}
		sup := ""
		if len(k.Supertraits) > 0 {
			sup = ": " + ast.BoundsString(k.Supertraits)
		}
		if p.harness && name == "Sized" {
			sup = ": crate::__harness::MetaSized"
		}
		gens := p.genericsDef(k.Generics)
		if p.harness && (name == "Fn" || name == "FnMut" || name == "FnOnce") {
			gens = "<Args: crate::marker::Tuple>"
		}
		p.w("%s%s%strait %s%s%s%s {\n", ind, vis, u, name, gens, sup, p.where(k.Generics))
		saved := p.trait
		p.trait = name
		for _, ti := range k.Items {
			p.item(ti, ind+"    ")
		}
		p.trait = saved
		p.w("%s}\n", ind)
	case *ast.ImplItem:
		// rustc implements Clone for tuples itself.
		if p.harness && k.Trait != nil && ast.PathString(k.Trait) == "Clone" && strings.HasPrefix(ast.TypeString(k.SelfTy), "(") {
			return
		}
		u := ""
		if k.Unsafe {
			u = "unsafe "
		}
		tr := ""
		if k.Trait != nil {
			neg := ""
			if k.Negative {
				neg = "!"
			}
			tr = neg + ast.PathString(k.Trait) + " for "
		}
		gens := p.generics(k.Generics)
		if p.harness && k.Trait != nil && ast.PathString(k.Trait) == "Drop" && len(k.Generics.Params) > 0 {
			// Like the real library: dropping a container does not use
			// borrows held by its elements.
			for _, gp := range k.Generics.Params {
				if gp.Kind == ast.GPType {
					gens = strings.Replace(gens, gp.Name.Name, "#[may_dangle] "+gp.Name.Name, 1)
					u = "unsafe "
					break
				}
			}
		}
		p.w("%s%simpl%s %s%s%s {\n", ind, u, gens, tr, ast.TypeString(k.SelfTy), p.where(k.Generics))
		saved := p.trait
		if k.Trait == nil {
			p.trait = "<" + strings.SplitN(ast.TypeString(k.SelfTy), "<", 2)[0] + ">"
		} else {
			p.trait = ""
		}
		for _, ii := range k.Items {
			p.item(ii, ind+"    ")
		}
		p.trait = saved
		p.w("%s}\n", ind)
	case *ast.TypeAliasItem:
		if p.harness {
			switch {
			case p.trait == "FnOnce" && name == "Output":
				p.w("%s#[lang = \"fn_once_output\"]\n", ind)
			case p.trait == "Deref" && name == "Target":
				p.w("%s#[lang = \"deref_target\"]\n", ind)
			}
		}
		b := ""
		if len(k.Bounds) > 0 {
			b = ": " + ast.BoundsString(k.Bounds)
		}
		if k.Ty == nil {
			p.w("%s%stype %s%s%s%s;\n", ind, vis, name, p.generics(k.Generics), b, p.where(k.Generics))
		} else {
			p.w("%s%stype %s%s%s%s = %s;\n", ind, vis, name, p.generics(k.Generics), b, p.where(k.Generics), ast.TypeString(k.Ty))
		}
	case *ast.ConstItem:
		if k.Expr == nil {
			p.w("%s%sconst %s: %s;\n", ind, vis, name, ast.TypeString(k.Ty))
		} else {
			p.w("%s%sconst %s: %s = %s;\n", ind, vis, name, ast.TypeString(k.Ty), ast.ExprString(k.Expr))
		}
	case *ast.StaticItem:
		m := ""
		if k.Mut {
			m = "mut "
		}
		p.w("%s%sstatic %s%s: %s = %s;\n", ind, vis, m, name, ast.TypeString(k.Ty), ast.ExprString(k.Expr))
	case *ast.ModItem:
		p.w("%s%smod %s {\n", ind, vis, name)
		p.items(k.Items, ind+"    ", name != "prelude" && !strings.HasPrefix(name, "rust_20") && name != "v1")
		p.w("%s}\n", ind)
	case *ast.UseItem:
		p.w("%s%suse %s;\n", ind, vis, p.useTree(k.Tree))
	case *ast.ExternCrateItem:
		p.w("%sextern crate %s;\n", ind, name)
	}
}

// harnessPrelude supplies what rustc's no_core mode needs on top of our
// lang items (Copy for primitives, unsizing, receivers, panic hooks).
const harnessPrelude = `#![feature(no_core, lang_items, rustc_attrs, auto_traits, negative_impls, unboxed_closures, fundamental, never_type, intrinsics, arbitrary_self_types, const_trait_impl, decl_macro, dropck_eyepatch)]
#![no_core]
#![rustc_coherence_is_core]
#![allow(unused, dead_code, internal_features, incomplete_features, improper_ctypes_definitions, unconditional_recursion, non_camel_case_types)]

mod __harness {
    use crate::marker::{Copy, Sized};
    #[lang = "pointee_sized"] pub trait PointeeSized {}
    #[lang = "meta_sized"] pub trait MetaSized: PointeeSized {}
    #[lang = "unsize"] pub trait Unsize<T: ?Sized> {}
    #[lang = "coerce_unsized"] pub trait CoerceUnsized<T: ?Sized> {}
    impl<'a, T: ?Sized + Unsize<U>, U: ?Sized> CoerceUnsized<&'a U> for &'a T {}
    impl<'a, T: ?Sized + Unsize<U>, U: ?Sized> CoerceUnsized<&'a mut U> for &'a mut T {}
    impl<T: ?Sized + Unsize<U>, U: ?Sized> CoerceUnsized<*const U> for *const T {}
    impl<T: ?Sized + Unsize<U>, U: ?Sized> CoerceUnsized<*mut U> for *mut T {}
    impl<T: ?Sized + Unsize<U>, U: ?Sized> CoerceUnsized<crate::boxed::Box<U>> for crate::boxed::Box<T> {}
    impl<T: ?Sized + Unsize<U>, U: ?Sized> CoerceUnsized<crate::rc::Rc<U>> for crate::rc::Rc<T> {}
    #[lang = "dispatch_from_dyn"] pub trait DispatchFromDyn<T> {}
    impl<'a, T: ?Sized + Unsize<U>, U: ?Sized> DispatchFromDyn<&'a U> for &'a T {}
    impl<'a, T: ?Sized + Unsize<U>, U: ?Sized> DispatchFromDyn<&'a mut U> for &'a mut T {}
    #[lang = "receiver"] pub trait Receiver { #[lang = "receiver_target"] type Target: ?Sized; }
    impl<P: ?Sized + crate::ops::Deref> Receiver for P { type Target = <P as crate::ops::Deref>::Target; }
    #[lang = "legacy_receiver"] pub trait LegacyReceiver {}
    impl<T: ?Sized> LegacyReceiver for &T {}
    impl<T: ?Sized> LegacyReceiver for &mut T {}
    impl<T: ?Sized> LegacyReceiver for crate::boxed::Box<T> {}
    #[lang = "freeze"] pub unsafe auto trait Freeze {}
    #[lang = "structural_peq"] pub trait StructuralPartialEq {}
    #[lang = "pointee_trait"] pub trait Pointee { #[lang = "metadata_type"] type Metadata: Copy; }
    #[lang = "discriminant_kind"] pub trait DiscriminantKind { #[lang = "discriminant_type"] type Discriminant; }
    #[lang = "destruct"] pub trait Destruct {}
    #[lang = "fn_ptr_trait"] pub trait FnPtr: Copy {}
    #[lang = "panic_location"] pub struct Location<'a> { file: &'a str, line: u32, col: u32 }
    #[lang = "panic"] pub fn panic(_: &'static str) -> ! { loop {} }
    #[lang = "panic_bounds_check"] pub fn panic_bounds_check(_: usize, _: usize) -> ! { loop {} }
    #[lang = "panic_cannot_unwind"] pub fn panic_cannot_unwind() -> ! { loop {} }
    #[lang = "panic_in_cleanup"] pub fn panic_in_cleanup() -> ! { loop {} }
    #[lang = "panic_misaligned_pointer_dereference"] pub fn pmpd(_: usize, _: usize) -> ! { loop {} }
    #[lang = "panic_const_add_overflow"] pub fn p1() -> ! { loop {} }
    #[lang = "panic_const_sub_overflow"] pub fn p2() -> ! { loop {} }
    #[lang = "panic_const_mul_overflow"] pub fn p3() -> ! { loop {} }
    #[lang = "panic_const_div_overflow"] pub fn p4() -> ! { loop {} }
    #[lang = "panic_const_rem_overflow"] pub fn p5() -> ! { loop {} }
    #[lang = "panic_const_neg_overflow"] pub fn p6() -> ! { loop {} }
    #[lang = "panic_const_shr_overflow"] pub fn p7() -> ! { loop {} }
    #[lang = "panic_const_shl_overflow"] pub fn p8() -> ! { loop {} }
    #[lang = "panic_const_div_by_zero"] pub fn p9() -> ! { loop {} }
    #[lang = "panic_const_rem_by_zero"] pub fn p10() -> ! { loop {} }
    // Box<F> implements the Fn traits (built into the rustgo compiler).
    impl<Args: crate::marker::Tuple, F: crate::ops::FnOnce<Args> + ?Sized> crate::ops::FnOnce<Args> for crate::boxed::Box<F> {
        type Output = <F as crate::ops::FnOnce<Args>>::Output;
        extern "rust-call" fn call_once(self, args: Args) -> Self::Output { loop {} }
    }
    impl<Args: crate::marker::Tuple, F: crate::ops::FnMut<Args> + ?Sized> crate::ops::FnMut<Args> for crate::boxed::Box<F> {
        extern "rust-call" fn call_mut(&mut self, args: Args) -> Self::Output { <F as crate::ops::FnMut<Args>>::call_mut(&mut **self, args) }
    }
    impl<Args: crate::marker::Tuple, F: crate::ops::Fn<Args> + ?Sized> crate::ops::Fn<Args> for crate::boxed::Box<F> {
        extern "rust-call" fn call(&self, args: Args) -> Self::Output { <F as crate::ops::Fn<Args>>::call(&**self, args) }
    }
    macro_rules! copy_impls { ($($t:ty)*) => { $( impl Copy for $t {} )* } }
    copy_impls! { i8 i16 i32 i64 i128 isize u8 u16 u32 u64 u128 usize f32 f64 bool char }
    impl<T: ?Sized> Copy for &T {}
    impl<T: Copy, const N: usize> Copy for [T; N] {}
    impl<T: ?Sized> Copy for *const T {}
    impl<T: ?Sized> Copy for *mut T {}
}
`

func TestRustcTypeCheck(t *testing.T) {
	if os.Getenv("RUSTGO_STD_RUSTC_CHECK") == "" {
		t.Skip("set RUSTGO_STD_RUSTC_CHECK=1 to type-check the library with rustc")
	}
	rustc, err := exec.LookPath("rustc")
	if err != nil {
		t.Skip("rustc not found")
	}
	c, err := LoadCrate(lexer.AddFile)
	if err != nil {
		t.Fatal(err)
	}
	if err := expand.New().ExpandCrate(c, "std"); err != nil {
		t.Fatal(err)
	}
	p := &itemPrinter{harness: true}
	p.items(c.Items, "", true)
	src := harnessPrelude + p.b.String()
	src = strings.ReplaceAll(src, "::std::", "crate::")
	// Macro-generated `#[doc = $doc]` leftovers and the like are harmless; drop
	// attributes rustc does not know in no_core mode.
	src = regexp.MustCompile(`(?m)^\s*#\[lang = "(Result|into_iterator|from|owned_box)"\]\n`).ReplaceAllString(src, "")
	dir := os.Getenv("RUSTGO_STD_RUSTC_OUT")
	if dir == "" {
		dir = t.TempDir()
	}
	file := filepath.Join(dir, "std_expanded.rs")
	if err := os.WriteFile(file, []byte(src), 0o644); err != nil {
		t.Fatal(err)
	}
	cmd := exec.Command(rustc, "--edition", "2021", "--crate-type", "lib", "--crate-name", "std", "--emit=metadata", "-o", filepath.Join(dir, "libstd.rmeta"), file)
	cmd.Env = append(os.Environ(), "RUSTC_BOOTSTRAP=1")
	out, err := cmd.CombinedOutput()
	if err != nil {
		lines := strings.Split(string(out), "\n")
		if len(lines) > 400 {
			lines = lines[:400]
		}
		t.Fatalf("rustc rejected the expanded library (%s):\n%s", file, strings.Join(lines, "\n"))
	}
}

// hoistBlockItems moves items declared inside function bodies to the
// enclosing item list (the AST printer cannot print nested items).
func hoistBlockItems(items []*ast.Item) []*ast.Item {
	var extra []*ast.Item
	var visit func(n any)
	visit = func(n any) {
		ast.Walk(n, func(x any) bool {
			if st, ok := x.(*ast.ItemStmt); ok && st.Item != nil {
				switch st.Item.Kind.(type) {
				case *ast.MacroRulesItem, *ast.MacCallItem:
				default:
					extra = append(extra, st.Item)
				}
			}
			return true
		})
	}
	for _, it := range items {
		switch k := it.Kind.(type) {
		case *ast.FnItem:
			if k.Body != nil {
				visit(k.Body)
			}
		case *ast.ImplItem:
			for _, ii := range k.Items {
				if f, ok := ii.Kind.(*ast.FnItem); ok && f.Body != nil {
					visit(f.Body)
				}
			}
		case *ast.ModItem:
			k.Items = hoistBlockItems(k.Items)
		}
	}
	return append(items, extra...)
}

// TestRustcTypeCheckPrograms type-checks every tests/run-pass program,
// expanded by our expander, against the rustc-checked library. Failures are
// reported per program (some are expected: moving out of a `Box` with `*b`
// needs the compiler's builtin box support, which the rustc harness lacks).
func TestRustcTypeCheckPrograms(t *testing.T) {
	if os.Getenv("RUSTGO_STD_RUSTC_CHECK") == "" {
		t.Skip("set RUSTGO_STD_RUSTC_CHECK=1 to type-check the test programs with rustc")
	}
	rustc, err := exec.LookPath("rustc")
	if err != nil {
		t.Skip("rustc not found")
	}
	dir := os.Getenv("RUSTGO_STD_RUSTC_OUT")
	if dir == "" {
		dir = t.TempDir()
	}
	e := expand.New()
	c, err := LoadCrate(lexer.AddFile)
	if err != nil {
		t.Fatal(err)
	}
	if err := e.ExpandCrate(c, "std"); err != nil {
		t.Fatal(err)
	}
	p := &itemPrinter{harness: true}
	p.items(c.Items, "", true)
	src := strings.ReplaceAll(harnessPrelude+p.b.String(), "::std::", "crate::")
	stdFile := filepath.Join(dir, "std_expanded.rs")
	rmeta := filepath.Join(dir, "libstd.rmeta")
	if err := os.WriteFile(stdFile, []byte(src), 0o644); err != nil {
		t.Fatal(err)
	}
	cmd := exec.Command(rustc, "--edition", "2021", "--crate-type", "lib", "--crate-name", "std", "--emit=metadata", "-o", rmeta, stdFile)
	cmd.Env = append(os.Environ(), "RUSTC_BOOTSTRAP=1")
	if out, err := cmd.CombinedOutput(); err != nil {
		t.Fatalf("library does not type-check:\n%s", out)
	}
	progs, _ := filepath.Glob("../../tests/run-pass/*.rs")
	if pat := os.Getenv("RUSTGO_STD_RUSTC_PROGS"); pat != "" {
		var keep []string
		for _, pr := range progs {
			if strings.Contains(pr, pat) {
				keep = append(keep, pr)
			}
		}
		progs = keep
	}
	failed := 0
	for _, prog := range progs {
		data, err := os.ReadFile(prog)
		if err != nil {
			t.Fatal(err)
		}
		uc, err := parserParseCrate(prog, string(data))
		if err != nil {
			t.Errorf("%s: parse: %v", prog, err)
			continue
		}
		if err := e.ExpandCrate(uc, "main"); err != nil {
			t.Errorf("%s: expand: %v", prog, err)
			continue
		}
		uc.Items = hoistBlockItems(uc.Items)
		up := &itemPrinter{}
		up.b.WriteString("#![feature(no_core, lang_items, unboxed_closures, never_type)]\n#![no_core]\n#![allow(unused, dead_code, non_snake_case, non_camel_case_types, unused_must_use, internal_features)]\nextern crate std;\n")
		up.items(uc.Items, "", false)
		usrc := up.b.String()
		usrc = strings.Replace(usrc, "extern crate std;\n", "extern crate std;\nextern crate std as core;\nextern crate std as alloc;\n", 1)
		// Every module sees the std prelude.
		usrc = regexp.MustCompile(`(?m)^extern crate std;\n`).ReplaceAllString(usrc, "extern crate std;\n#[allow(unused_imports)]\nuse std::prelude::rust_2021::*;\n")
		usrc = regexp.MustCompile(`(?m)^(\s*)((pub(\(crate\))? )?mod \w+ \{)\n`).ReplaceAllString(usrc, "$1$2\n$1    #[allow(unused_imports)]\n$1    use std::prelude::rust_2021::*;\n")
		base := strings.TrimSuffix(filepath.Base(prog), ".rs")
		ufile := filepath.Join(dir, base+"_expanded.rs")
		if err := os.WriteFile(ufile, []byte(usrc), 0o644); err != nil {
			t.Fatal(err)
		}
		cmd := exec.Command(rustc, "--edition", "2021", "--crate-type", "lib", "--crate-name", "prog_"+base, "--emit=metadata",
			"-o", filepath.Join(dir, base+".rmeta"), "--extern", "std="+rmeta, ufile)
		cmd.Env = append(os.Environ(), "RUSTC_BOOTSTRAP=1")
		if out, err := cmd.CombinedOutput(); err != nil {
			failed++
			lines := strings.Split(string(out), "\n")
			if len(lines) > 60 {
				lines = lines[:60]
			}
			t.Logf("%s: rustc errors:\n%s", base, strings.Join(lines, "\n"))
		}
	}
	if failed > 0 {
		t.Errorf("%d of %d programs failed to type-check against the library", failed, len(progs))
	}
}
