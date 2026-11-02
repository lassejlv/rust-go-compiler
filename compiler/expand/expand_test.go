package expand

import (
	"fmt"
	"os"
	"path/filepath"
	"rustgo/compiler/stdlib"
	"strings"
	"testing"

	"rustgo/compiler/ast"
	"rustgo/compiler/lexer"
	"rustgo/compiler/parser"
)

// ---------------------------------------------------------------------------
// Helpers

func parseCrate(t *testing.T, name, src string) *ast.Crate {
	t.Helper()
	f := lexer.AddFile(name, src)
	c, err := parser.ParseCrate(f, src)
	if err != nil {
		t.Fatalf("parse: %v", err)
	}
	return c
}

func expandSrcErr(t *testing.T, src string) (*ast.Crate, error) {
	t.Helper()
	c := parseCrate(t, "test.rs", src)
	err := New().ExpandCrate(c, "main")
	return c, err
}

func expandSrc(t *testing.T, src string) *ast.Crate {
	t.Helper()
	c, err := expandSrcErr(t, src)
	if err != nil {
		t.Fatalf("expand: %v", err)
	}
	return c
}

func mustErr(t *testing.T, src, want string) {
	t.Helper()
	_, err := expandSrcErr(t, src)
	if err == nil {
		t.Fatalf("expected error containing %q, got none", want)
	}
	if !strings.Contains(err.Error(), want) {
		t.Fatalf("expected error containing %q, got %q", want, err.Error())
	}
	if !strings.Contains(err.Error(), ": error: ") {
		t.Fatalf("error not in `file:line:col: error: msg` form: %q", err.Error())
	}
}

func findItem(items []*ast.Item, name string) *ast.Item {
	for _, it := range items {
		if it.Name.Name == name {
			return it
		}
	}
	return nil
}

func fnBody(t *testing.T, c *ast.Crate, name string) string {
	t.Helper()
	it := findItem(c.Items, name)
	if it == nil {
		t.Fatalf("no item %s", name)
	}
	return ast.BlockString(it.Kind.(*ast.FnItem).Body)
}

// expandExpr expands `fn main() { let __r = SRC; }` (plus extra items) and
// returns the printed initializer.
func expandExprSrc(t *testing.T, extra, src string) string {
	t.Helper()
	c := expandSrc(t, extra+"\nfn main() { let __r = "+src+"; }")
	body := findItem(c.Items, "main").Kind.(*ast.FnItem).Body
	for _, s := range body.Stmts {
		if ls, ok := s.(*ast.LetStmt); ok {
			return ast.ExprString(ls.Init)
		}
	}
	t.Fatalf("no let in %s", ast.BlockString(body))
	return ""
}

func fnString(name string, k *ast.FnItem) string {
	var params []string
	if sp := k.Decl.SelfParam; sp != nil {
		switch sp.Kind {
		case ast.SelfRef:
			params = append(params, "&self")
		case ast.SelfRefMut:
			params = append(params, "&mut self")
		default:
			params = append(params, "self")
		}
	}
	for _, p := range k.Decl.Params {
		params = append(params, ast.PatString(p.Pat)+": "+ast.TypeString(p.Ty))
	}
	s := "fn " + name + ast.GenericsParamsString(k.Generics) + "(" + strings.Join(params, ", ") + ")"
	if k.Decl.Output != nil {
		s += " -> " + ast.TypeString(k.Decl.Output)
	}
	if k.Body != nil {
		s += " " + ast.BlockString(k.Body)
	}
	return s
}

// itemString is a small item printer for tests.
func itemString(it *ast.Item) string {
	switch k := it.Kind.(type) {
	case *ast.FnItem:
		return fnString(it.Name.Name, k)
	case *ast.ImplItem:
		s := "impl" + ast.GenericsParamsString(k.Generics) + " "
		if k.Trait != nil {
			s += ast.PathString(k.Trait) + " for "
		}
		s += ast.TypeString(k.SelfTy)
		if w := ast.WhereString(k.Generics); w != "" {
			s += " " + w
		}
		var parts []string
		for _, m := range k.Items {
			parts = append(parts, itemString(m))
		}
		return s + " { " + strings.Join(parts, " ") + " }"
	case *ast.StructItem:
		var fs []string
		for _, f := range k.Fields {
			fs = append(fs, f.Name.Name)
		}
		return "struct " + it.Name.Name + "{" + strings.Join(fs, ",") + "}"
	case *ast.EnumItem:
		var vs []string
		for _, v := range k.Variants {
			vs = append(vs, v.Name.Name)
		}
		return "enum " + it.Name.Name + "{" + strings.Join(vs, ",") + "}"
	case *ast.ModItem:
		var parts []string
		for _, m := range k.Items {
			parts = append(parts, itemString(m))
		}
		return "mod " + it.Name.Name + " { " + strings.Join(parts, " ") + " }"
	case *ast.ConstItem:
		return "const " + it.Name.Name + ": " + ast.TypeString(k.Ty) + " = " + ast.ExprString(k.Expr) + ";"
	case *ast.StaticItem:
		return "static " + it.Name.Name + ": " + ast.TypeString(k.Ty) + " = " + ast.ExprString(k.Expr) + ";"
	case *ast.TypeAliasItem:
		return "type " + it.Name.Name + " = " + ast.TypeString(k.Ty) + ";"
	}
	return fmt.Sprintf("<%T %s>", it.Kind, it.Name.Name)
}

func crateString(c *ast.Crate) string {
	var parts []string
	for _, it := range c.Items {
		parts = append(parts, itemString(it))
	}
	return strings.Join(parts, "\n")
}

func contains(t *testing.T, got string, wants ...string) {
	t.Helper()
	for _, w := range wants {
		if !strings.Contains(got, w) {
			t.Errorf("output does not contain %q:\n%s", w, got)
		}
	}
}

func notContains(t *testing.T, got string, bad ...string) {
	t.Helper()
	for _, w := range bad {
		if strings.Contains(got, w) {
			t.Errorf("output unexpectedly contains %q:\n%s", w, got)
		}
	}
}

// noMacros asserts that no macro invocation remains in the crate.
func noMacros(t *testing.T, c *ast.Crate) {
	t.Helper()
	ast.Walk(c, func(n any) bool {
		switch x := n.(type) {
		case *ast.MacCallExpr, *ast.MacStmt, *ast.MacPat, *ast.MacroType:
			t.Errorf("macro left after expansion: %T", n)
		case *ast.Item:
			if _, ok := x.Kind.(*ast.MacCallItem); ok {
				t.Errorf("item macro left after expansion")
			}
		}
		return true
	})
}

// ---------------------------------------------------------------------------
// Format string parser

func phString(p *FmtPlaceholder) string {
	arg := ""
	switch p.Arg.Kind {
	case FmtArgIndex:
		arg = fmt.Sprint(p.Arg.Index)
		if p.Arg.Implicit {
			arg += "i"
		}
	case FmtArgName:
		arg = p.Arg.Name
	}
	cnt := func(c FmtCount) string {
		switch c.Kind {
		case FmtCountIs:
			return fmt.Sprint(c.N)
		case FmtCountParam:
			if c.Ref.Kind == FmtArgName {
				return c.Ref.Name + "$"
			}
			return fmt.Sprintf("%d$", c.Ref.Index)
		}
		return "-"
	}
	fill := ""
	if p.Fill != 0 {
		fill = string(p.Fill)
	}
	flags := ""
	if p.Plus {
		flags += "+"
	}
	if p.Minus {
		flags += "-"
	}
	if p.Alternate {
		flags += "#"
	}
	if p.ZeroPad {
		flags += "0"
	}
	if p.DebugHex != 0 {
		flags += string(p.DebugHex) + "?"
	}
	al := ""
	if p.Align != 0 {
		al = string(p.Align)
	}
	return fmt.Sprintf("{%s %s fill=%s al=%s fl=%s w=%s p=%s}", arg, p.Trait, fill, al, flags, cnt(p.Width), cnt(p.Prec))
}

func piecesString(ps []FmtPiece) string {
	var b strings.Builder
	for _, p := range ps {
		if p.Ph == nil {
			fmt.Fprintf(&b, "%q", p.Lit)
		} else {
			b.WriteString(phString(p.Ph))
		}
	}
	return b.String()
}

func TestParseFormatString(t *testing.T) {
	cases := []struct{ in, want string }{
		{"hello", `"hello"`},
		{"", ``},
		{"{}", `{0i Display fill= al= fl= w=- p=-}`},
		{"a{}b{}c", `"a"{0i Display fill= al= fl= w=- p=-}"b"{1i Display fill= al= fl= w=- p=-}"c"`},
		{"{:?}", `{0i Debug fill= al= fl= w=- p=-}`},
		{"{0} {1} {0}", `{0 Display fill= al= fl= w=- p=-}" "{1 Display fill= al= fl= w=- p=-}" "{0 Display fill= al= fl= w=- p=-}`},
		{"{name}", `{name Display fill= al= fl= w=- p=-}`},
		{"{{}}", `"{}"`},
		{"{{{}}}", `"{"{0i Display fill= al= fl= w=- p=-}"}"`},
		{"{:>8}", `{0i Display fill= al=> fl= w=8 p=-}`},
		{"{:*^10}", `{0i Display fill=* al=^ fl= w=10 p=-}`},
		{"{:<}", `{0i Display fill= al=< fl= w=- p=-}`},
		{"{:+.3}", `{0i Display fill= al= fl=+ w=- p=3}`},
		{"{:08.2}", `{0i Display fill= al= fl=0 w=8 p=2}`},
		{"{:#?}", `{0i Debug fill= al= fl=# w=- p=-}`},
		{"{:#x}", `{0i LowerHex fill= al= fl=# w=- p=-}`},
		{"{:#010b}", `{0i Binary fill= al= fl=#0 w=10 p=-}`},
		{"{:x?}", `{0i Debug fill= al= fl=x? w=- p=-}`},
		{"{:X?}", `{0i Debug fill= al= fl=X? w=- p=-}`},
		{"{:X}", `{0i UpperHex fill= al= fl= w=- p=-}`},
		{"{:o}", `{0i Octal fill= al= fl= w=- p=-}`},
		{"{:e}", `{0i LowerExp fill= al= fl= w=- p=-}`},
		{"{:E}", `{0i UpperExp fill= al= fl= w=- p=-}`},
		{"{:p}", `{0i Pointer fill= al= fl= w=- p=-}`},
		{"{:>width$}", `{0i Display fill= al=> fl= w=width$ p=-}`},
		{"{:1$}", `{0i Display fill= al= fl= w=1$ p=-}`},
		{"{:0$}", `{0i Display fill= al= fl= w=0$ p=-}`},
		{"{:.prec$}", `{0i Display fill= al= fl= w=- p=prec$}`},
		{"{:.*}", `{1i Display fill= al= fl= w=- p=0$}`},
		{"{} {:.*} {}", `{0i Display fill= al= fl= w=- p=-}" "{2i Display fill= al= fl= w=- p=1$}" "{3i Display fill= al= fl= w=- p=-}`},
		{"{0:>5}", `{0 Display fill= al=> fl= w=5 p=-}`},
		{"{name:?}", `{name Debug fill= al= fl= w=- p=-}`},
		{"{:-^+#08.3x}", `{0i LowerHex fill=- al=^ fl=+#0 w=8 p=3}`},
		{"{:é>4}", `{0i Display fill=é al=> fl= w=4 p=-}`},
		{"{:}", `{0i Display fill= al= fl= w=- p=-}`},
		{"{x:width$.prec$}", `{x Display fill= al= fl= w=width$ p=prec$}`},
		{"é{}ü", `"é"{0i Display fill= al= fl= w=- p=-}"ü"`},
	}
	for _, c := range cases {
		ps, err := ParseFormatString(c.in)
		if err != nil {
			t.Errorf("%q: unexpected error %v", c.in, err)
			continue
		}
		if got := piecesString(ps); got != c.want {
			t.Errorf("%q:\n got %s\nwant %s", c.in, got, c.want)
		}
	}
}

func TestParseFormatStringErrors(t *testing.T) {
	cases := []struct{ in, want string }{
		{"{", "expected `}` but string was terminated"},
		{"}", "unmatched `}` found"},
		{"a } b", "unmatched `}` found"},
		{"{0", "expected `}` but string was terminated"},
		{"{:q}", "unknown format trait `q`"},
		{"{:.}", "expected precision after `.`"},
		{"{ }", "expected `}`"},
		{"{_}", "invalid argument name `_`"},
		{"{:?x}", "expected `}`"},
	}
	for _, c := range cases {
		_, err := ParseFormatString(c.in)
		if err == nil {
			t.Errorf("%q: expected error", c.in)
			continue
		}
		if !strings.Contains(err.Error(), c.want) || !strings.HasPrefix(err.Error(), "invalid format string: ") {
			t.Errorf("%q: got error %q, want %q", c.in, err.Error(), c.want)
		}
	}
}

// ---------------------------------------------------------------------------
// format_args! and friends

const specNone = "::std::option::Option::None"

func TestFormatArgsBasic(t *testing.T) {
	got := expandExprSrc(t, "", `format_args!("a{}b{:?}", x + 1, y)`)
	want := `::std::fmt::Arguments::new({ let mut __fmt = ::std::fmt::Formatter::new(); match (&(x + 1), &y) { (__arg0, __arg1) => { __fmt.write_str("a"); __fmt.set_spec(' ', 0u8, 0u32, ` + specNone + `, ` + specNone + `); let _ = ::std::fmt::Display::fmt(__arg0, &mut __fmt); __fmt.write_str("b"); __fmt.set_spec(' ', 0u8, 0u32, ` + specNone + `, ` + specNone + `); let _ = ::std::fmt::Debug::fmt(__arg1, &mut __fmt); }, }; __fmt.into_string() })`
	if got != want {
		t.Errorf("got\n%s\nwant\n%s", got, want)
	}
}

func TestFormatArgsNoArgs(t *testing.T) {
	got := expandExprSrc(t, "", `format_args!("hi {{x}}\n")`)
	want := `::std::fmt::Arguments::new(::std::string::String::from("hi {x}\n"))`
	if got != want {
		t.Errorf("got %s want %s", got, want)
	}
	// raw string format strings
	got = expandExprSrc(t, "", `format_args!(r#"say "{}""#, 1,)`)
	contains(t, got, `__fmt.write_str("say \"")`, `__fmt.write_str("\"")`, `match (&1,)`)
}

func TestFormatSpecs(t *testing.T) {
	cases := []struct{ src, want string }{
		{`format_args!("{:>8}", x)`, `set_spec(' ', 2u8, 0u32, ::std::option::Option::Some(8usize), ` + specNone + `)`},
		{`format_args!("{:*^10}", x)`, `set_spec('*', 3u8, 0u32, ::std::option::Option::Some(10usize), ` + specNone + `)`},
		{`format_args!("{:<5}", x)`, `set_spec(' ', 1u8, 0u32, ::std::option::Option::Some(5usize), ` + specNone + `)`},
		{`format_args!("{:+.3}", x)`, `set_spec(' ', 0u8, 1u32, ` + specNone + `, ::std::option::Option::Some(3usize))`},
		{`format_args!("{:-}", x)`, `set_spec(' ', 0u8, 2u32,`},
		{`format_args!("{:08.2}", x)`, `set_spec(' ', 0u8, 8u32, ::std::option::Option::Some(8usize), ::std::option::Option::Some(2usize))`},
		{`format_args!("{:#?}", x)`, `set_spec(' ', 0u8, 4u32, ` + specNone + `, ` + specNone + `); let _ = ::std::fmt::Debug::fmt(__arg0`},
		{`format_args!("{:#x}", x)`, `set_spec(' ', 0u8, 4u32, ` + specNone + `, ` + specNone + `); let _ = ::std::fmt::LowerHex::fmt(__arg0`},
		{`format_args!("{:#010b}", x)`, `set_spec(' ', 0u8, 12u32, ::std::option::Option::Some(10usize), ` + specNone + `); let _ = ::std::fmt::Binary::fmt(`},
		{`format_args!("{:x?}", x)`, `set_spec(' ', 0u8, 16u32, ` + specNone + `, ` + specNone + `); let _ = ::std::fmt::Debug::fmt(`},
		{`format_args!("{:X?}", x)`, `set_spec(' ', 0u8, 32u32,`},
		{`format_args!("{:X}", x)`, `::std::fmt::UpperHex::fmt(`},
		{`format_args!("{:o}", x)`, `::std::fmt::Octal::fmt(`},
		{`format_args!("{:e}", x)`, `::std::fmt::LowerExp::fmt(`},
		{`format_args!("{:E}", x)`, `::std::fmt::UpperExp::fmt(`},
		{`format_args!("{:p}", x)`, `::std::fmt::Pointer::fmt(`},
		{`format_args!("{:'>4}", x)`, `set_spec('\'', 2u8`},
		{`format_args!("{:\\<4}", x)`, `set_spec('\\', 1u8`},
	}
	for _, c := range cases {
		got := expandExprSrc(t, "", c.src)
		if !strings.Contains(got, c.want) {
			t.Errorf("%s:\n got %s\nwant substring %s", c.src, got, c.want)
		}
	}
}

func TestFormatArgRefs(t *testing.T) {
	// width from named arg
	got := expandExprSrc(t, "", `format_args!("{:>width$}", x, width = 6)`)
	contains(t, got, "match (&x, &6) { (__arg0, __arg1)", "set_spec(' ', 2u8, 0u32, ::std::option::Option::Some(*__arg1), "+specNone+"); let _ = ::std::fmt::Display::fmt(__arg0")
	// positional width
	got = expandExprSrc(t, "", `format_args!("{:1$}", x, 7)`)
	contains(t, got, "::std::option::Option::Some(*__arg1), "+specNone+"); let _ = ::std::fmt::Display::fmt(__arg0")
	// .prec$
	got = expandExprSrc(t, "", `format_args!("{:.prec$}", 1.5, prec = 2)`)
	contains(t, got, specNone+", ::std::option::Option::Some(*__arg1)); let _ = ::std::fmt::Display::fmt(__arg0")
	// .* takes two args: precision, then the value
	got = expandExprSrc(t, "", `format_args!("{} {:.*} {}", a, 3, b, c)`)
	contains(t, got, "match (&a, &3, &b, &c)",
		"::std::fmt::Display::fmt(__arg0,",
		specNone+", ::std::option::Option::Some(*__arg1)); let _ = ::std::fmt::Display::fmt(__arg2,",
		"::std::fmt::Display::fmt(__arg3,")
	// explicit index, reuse of the same argument
	got = expandExprSrc(t, "", `format_args!("{0} {0:>5} {1:?} {0}", a, b)`)
	contains(t, got, "match (&a, &b)", "Display::fmt(__arg0", "Some(5usize)", "Debug::fmt(__arg1")
	if strings.Count(got, "__arg0, &mut __fmt") != 3 {
		t.Errorf("expected 3 uses of __arg0: %s", got)
	}
	// named args, implicit capture, named debug
	got = expandExprSrc(t, "", `format_args!("{name} {other:?} {name}", name = n + 1)`)
	contains(t, got, "match (&(n + 1), &other) { (__arg0, __arg1)", "Debug::fmt(__arg1")
	// implicit capture used in width
	got = expandExprSrc(t, "", `format_args!("{:w$}", v)`)
	contains(t, got, "match (&v, &w)", "Some(*__arg1)")
	// named argument referenced positionally
	got = expandExprSrc(t, "", `format_args!("{} {}", 1, x = 2)`)
	contains(t, got, "match (&1, &2)")
}

func TestFormatArgsErrors(t *testing.T) {
	mustErr(t, `fn main() { format_args!("{}", 1, 2); }`, "argument never used")
	mustErr(t, `fn main() { format_args!("{}", 1, x = 2); }`, "named argument never used")
	mustErr(t, `fn main() { format_args!("{} {}", 1); }`, "invalid reference to positional argument 1 (there is 1 argument)")
	mustErr(t, `fn main() { format_args!("{5}", 1, 2); }`, "invalid reference to positional argument 5 (there are 2 arguments)")
	mustErr(t, `fn main() { format_args!("{}"); }`, "1 positional argument in format string, but no arguments were given")
	mustErr(t, `fn main() { format_args!("{:q}", 1); }`, "unknown format trait `q`")
	mustErr(t, `fn main() { format_args!("{", 1); }`, "invalid format string")
	mustErr(t, `fn main() { format_args!(x, 1); }`, "format argument must be a string literal")
	mustErr(t, `fn main() { format_args!(); }`, "requires at least a format string argument")
	mustErr(t, `fn main() { format_args!("{x}", x = 1, x = 2); }`, "duplicate argument named `x`")
	mustErr(t, `fn main() { format_args!("{x} {}", x = 1, 2); }`, "positional arguments cannot follow named arguments")
	mustErr(t, `fn main() { println!("{}" 1); }`, "expected `,`")
	// error location points into the user's source
	_, err := expandSrcErr(t, "fn main() {\n    println!(\"{}\", 1, 2);\n}")
	if err == nil || !strings.HasPrefix(err.Error(), "test.rs:2:") {
		t.Errorf("bad error location: %v", err)
	}
}

func TestFormatArgsConcatFormatString(t *testing.T) {
	got := expandExprSrc(t, "", `format!(concat!("a", "{}"), 5)`)
	contains(t, got, `__fmt.write_str("a")`, "match (&5,)")
}

func TestPrintMacros(t *testing.T) {
	c := expandSrc(t, `fn main() {
    print!("a{}", 1);
    println!("b{}", 2);
    println!();
    eprint!("c");
    eprintln!("d {x}");
    let s = format!("{}-{}", 1, 2);
}`)
	noMacros(t, c)
	b := fnBody(t, c, "main")
	contains(t, b,
		`::std::io::_print(::std::fmt::Arguments::new({ let mut __fmt = ::std::fmt::Formatter::new(); match (&1,) { (__arg0,) => { __fmt.write_str("a");`,
		`__fmt.write_str("b"); __fmt.set_spec`,
		`let _ = ::std::fmt::Display::fmt(__arg0, &mut __fmt); __fmt.write_str("\n"); }`,
		`::std::io::_print(::std::fmt::Arguments::new(::std::string::String::from("\n")));`,
		`::std::io::_eprint(::std::fmt::Arguments::new(::std::string::String::from("c")));`,
		`::std::io::_eprint(::std::fmt::Arguments::new({ let mut __fmt = ::std::fmt::Formatter::new(); match (&x,)`,
		`let s = ::std::fmt::format(::std::fmt::Arguments::new({`,
	)
	mustErr(t, `fn main() { print!(); }`, "requires at least a format string argument")
}

func TestWriteMacros(t *testing.T) {
	got := expandExprSrc(t, "", `write!(f, "{}", x)`)
	if !strings.HasPrefix(got, "f.write_fmt(::std::fmt::Arguments::new({") {
		t.Errorf("write!: %s", got)
	}
	got = expandExprSrc(t, "", `writeln!(self.out, "v={}", x)`)
	contains(t, got, "(self.out).write_fmt(", `__fmt.write_str("v=")`, `__fmt.write_str("\n")`)
	got = expandExprSrc(t, "", `writeln!(f)`)
	if got != `f.write_fmt(::std::fmt::Arguments::new(::std::string::String::from("\n")))` {
		t.Errorf("writeln!(f): %s", got)
	}
	got = expandExprSrc(t, "", `write!(&mut s, "x")`)
	contains(t, got, "(&mut s).write_fmt(")
}

// ---------------------------------------------------------------------------
// panic! family

func TestPanicMacros(t *testing.T) {
	src := "fn main() {\n    panic!();\n    panic!(\"boom {}\", 1);\n    unreachable!();\n    unreachable!(\"x={}\", x);\n    todo!();\n    todo!(\"later\");\n    unimplemented!();\n    unimplemented!(\"nope {}\", 2);\n    panic!(\"plain\");\n}"
	c := expandSrc(t, src)
	b := fnBody(t, c, "main")
	contains(t, b,
		`::std::rt::panic_str("explicit panic", "test.rs", 2, 5);`,
		`::std::rt::panic_fmt(::std::fmt::Arguments::new({ let mut __fmt = ::std::fmt::Formatter::new(); match (&1,) { (__arg0,) => { __fmt.write_str("boom ");`,
		`"test.rs", 3, 5);`,
		`::std::rt::panic_str("internal error: entered unreachable code", "test.rs", 4, 5);`,
		`__fmt.write_str("internal error: entered unreachable code: x=");`,
		`::std::rt::panic_str("not yet implemented", "test.rs", 6, 5);`,
		`::std::rt::panic_fmt(::std::fmt::Arguments::new(::std::string::String::from("not yet implemented: later")), "test.rs", 7, 5);`,
		`::std::rt::panic_str("not implemented", "test.rs", 8, 5);`,
		`__fmt.write_str("not implemented: nope ");`,
		`::std::rt::panic_fmt(::std::fmt::Arguments::new(::std::string::String::from("plain")), "test.rs", 10, 5);`,
	)
	mustErr(t, `fn main() { panic!(msg); }`, "format argument must be a string literal")
}

func TestPanicInExpressionPosition(t *testing.T) {
	got := expandExprSrc(t, "", `match x { 1 => 2, _ => panic!("bad {}", x) }`)
	contains(t, got, "_ => ::std::rt::panic_fmt(")
}

// ---------------------------------------------------------------------------
// assert! family

func TestAssertMacros(t *testing.T) {
	src := "fn main() {\n    assert!(v.len()>0);\n    assert!(a == b, \"a={} b={}\", a, b);\n    assert_eq!(x + 1, 2);\n    assert_ne!(x, y, \"msg {}\", z);\n    debug_assert!(ok);\n    debug_assert_eq!(1, 1,);\n    debug_assert_ne!(1, 2);\n}"
	c := expandSrc(t, src)
	noMacros(t, c)
	b := fnBody(t, c, "main")
	contains(t, b,
		`if !(v.len() > 0) { ::std::rt::panic_str("assertion failed: v.len() > 0", "test.rs", 2, 5) }`,
		`if !(a == b) { ::std::rt::panic_fmt(::std::fmt::Arguments::new({`,
		`__fmt.write_str("a=");`,
		`match (&(x + 1), &(2)) { (left_val, right_val) => { if !(*left_val == *right_val) { ::std::rt::assert_failed("==", &*left_val, &*right_val, ::std::option::Option::None, "test.rs", 4, 5); } }, }`,
		`match (&(x), &(y)) { (left_val, right_val) => { if *left_val == *right_val { ::std::rt::assert_failed("!=", &*left_val, &*right_val, ::std::option::Option::Some(::std::fmt::Arguments::new({`,
		`"test.rs", 5, 5); } }, }`,
		`if !(ok) { ::std::rt::panic_str("assertion failed: ok", "test.rs", 6, 5) }`,
		`assert_failed("==", &*left_val, &*right_val, ::std::option::Option::None, "test.rs", 7, 5)`,
		`assert_failed("!=", &*left_val, &*right_val, ::std::option::Option::None, "test.rs", 8, 5)`,
	)
	mustErr(t, `fn main() { assert_eq!(1); }`, "requires two expression arguments")
	mustErr(t, `fn main() { assert!(); }`, "requires an expression argument")
}

func TestAssertConditionText(t *testing.T) {
	got := expandExprSrc(t, "", `assert!(!v.is_empty() && x[0] >= -1)`)
	contains(t, got, `"assertion failed: !v.is_empty() && x[0] >= -1"`)
	got = expandExprSrc(t, "", `assert!(s == "a\"b")`)
	contains(t, got, `assertion failed: s == \"a\\\"b\"`)
}

// ---------------------------------------------------------------------------
// vec!, matches!, dbg!

func TestVec(t *testing.T) {
	cases := []struct{ src, want string }{
		{`vec![]`, `::std::vec::Vec::new()`},
		{`vec![1, 2, 3]`, `::std::vec::from_array([1, 2, 3])`},
		{`vec![a + b,]`, `::std::vec::from_array([a + b])`},
		{`vec![0u8; n * 2]`, `::std::vec::from_elem(0u8, n * 2)`},
		{`vec![|a, b| a + b]`, `::std::vec::from_array([|a, b| a + b])`},
		{`vec![HashMap::<i32, i32>::new(), m]`, `::std::vec::from_array([HashMap::<i32, i32>::new(), m])`},
		{`vec![vec![1], vec![]]`, `::std::vec::from_array([::std::vec::from_array([1]), ::std::vec::Vec::new()])`},
		{`std::vec![1]`, `::std::vec::from_array([1])`},
	}
	for _, c := range cases {
		got := expandExprSrc(t, "", c.src)
		if !strings.Contains(got, c.want) {
			t.Errorf("%s:\n got %s\nwant %s", c.src, got, c.want)
		}
	}
}

func TestMatches(t *testing.T) {
	got := expandExprSrc(t, "", `matches!(x, Some(1) | None)`)
	if got != `match x { Some(1) | None => true, _ => false, }` {
		t.Errorf("got %s", got)
	}
	got = expandExprSrc(t, "", `matches!(self.kind(), Kind::A { .. } if n > 2,)`)
	contains(t, got, `match (self.kind()) { Kind::A { .. } if n > 2 => true, _ => false, }`)
	got = expandExprSrc(t, "", `matches!(c, 'a'..='z' | '_')`)
	contains(t, got, `'a'..='z' | '_' => true`)
}

func TestDbg(t *testing.T) {
	src := "fn main() {\n    let a = dbg!(x * 2);\n    dbg!();\n    let (p, q) = dbg!(1, y);\n}"
	c := expandSrc(t, src)
	noMacros(t, c)
	b := fnBody(t, c, "main")
	contains(t, b,
		`let a = match (x * 2) { tmp => { ::std::io::_eprint(::std::fmt::Arguments::new({ let mut __fmt = ::std::fmt::Formatter::new(); match (&(&tmp),) { (__arg0,) => { __fmt.write_str("[test.rs:2:13] x * 2 = "); __fmt.set_spec(' ', 0u8, 4u32, `+specNone+`, `+specNone+`); let _ = ::std::fmt::Debug::fmt(__arg0, &mut __fmt); __fmt.write_str("\n"); }, }; __fmt.into_string() })); tmp }, };`,
		`::std::io::_eprint(::std::fmt::Arguments::new(::std::string::String::from("[test.rs:3:5]\n")));`,
		`let (p, q) = (match 1 { tmp =>`,
		`"[test.rs:4:18] 1 = "`,
		`"[test.rs:4:18] y = "`,
	)
}

// ---------------------------------------------------------------------------
// Literal builtins

func TestLiteralBuiltins(t *testing.T) {
	src := "fn main() {\n  let l = line!();\n    let c = column!();\n  let f = file!();\n  let s = stringify!(a + b);\n  let s2 = stringify!(foo(x, y).bar[0]);\n  let s3 = stringify!(a+b);\n  let k = concat!(\"a\", 1, 'c', true, -2, 1.5, stringify!(z));\n  let m = module_path!();\n  let d = cfg!(debug_assertions);\n  let w = cfg!(windows);\n  let o = option_env!(\"X\");\n  let lc = concat!(file!(), \":\", line!());\n}"
	c := expandSrc(t, src)
	b := fnBody(t, c, "main")
	contains(t, b,
		`let l = 2u32;`,
		`let c = column!();`[:0]+`let c = 13u32;`,
		`let f = "test.rs";`,
		`let s = "a + b";`,
		`let s2 = "foo(x, y).bar[0]";`,
		`let s3 = "a+b";`,
		`let k = "a1ctrue-21.5z";`,
		`let m = "main";`,
		`let d = true;`,
		`let w = false;`,
		`let o = ::std::option::Option::None::<&'static str>;`,
		`let lc = "test.rs:13";`,
	)
	c = expandSrc(t, "mod a { mod b { fn f() { let m = module_path!(); } } }")
	s := crateString(c)
	contains(t, s, `let m = "main::a::b";`)

	mustErr(t, `fn main() { let x = env!("HOME"); }`, "environment variable `HOME` not defined")
	mustErr(t, `fn main() { let x = include_str!("a.txt"); }`, "`include_str!` is not supported")
	mustErr(t, `compile_error!("custom failure");`, "custom failure")
	mustErr(t, `fn main() { compile_error!("in fn"); }`, "in fn")
	mustErr(t, `fn main() { nosuch!(1); }`, "cannot find macro `nosuch` in this scope")
}

func TestStringify(t *testing.T) {
	cases := []struct{ src, want string }{
		{`stringify!(a + b)`, `"a + b"`},
		{`stringify!(Vec<T>)`, `"Vec<T>"`},
		{`stringify!(fn foo(x: &mut i32) -> u8 { x })`, `"fn foo(x: &mut i32) -> u8 { x }"`},
		{`stringify!(#[attr] struct S;)`, `"#[attr] struct S;"`},
		{`stringify!("str" 'c' 1u8 r"raw")`, `"\"str\" 'c' 1u8 r\"raw\""`},
		{`stringify!()`, `""`},
		{`stringify!(a  ,   b)`, `"a, b"`},
	}
	for _, c := range cases {
		if got := expandExprSrc(t, "", c.src); got != c.want {
			t.Errorf("%s: got %s want %s", c.src, got, c.want)
		}
	}
}

func TestTokensStringSynthesized(t *testing.T) {
	sp := lexer.Span{}
	tts := []ast.TokenTree{
		identTok("f", sp),
		group('(', sp, []ast.TokenTree{identTok("x", sp), punctTok(",", sp, false), strTok("a\"b", sp)}),
		punctTok(".", sp, false),
		identTok("g", sp),
		group('{', sp, []ast.TokenTree{identTok("y", sp)}),
		punctTok(";", sp, false),
	}
	if got := TokensString(tts); got != `f(x, "a\"b").g { y };` {
		t.Errorf("got %s", got)
	}
}

// ---------------------------------------------------------------------------
// macro_rules!

func TestMacroRulesBasic(t *testing.T) {
	got := expandExprSrc(t, `macro_rules! double { ($e:expr) => { $e * 2 }; }`, `double!(1 + 2)`)
	if got != "(1 + 2) * 2" {
		t.Errorf("grouping not preserved: %s", got)
	}
	got = expandExprSrc(t, `
macro_rules! pick {
    (a) => { "first" };
    (b $x:expr) => { $x };
    ($($t:tt)*) => { "other" };
}`, `(pick!(a), pick!(b 5), pick!(c d))`)
	if got != `("first", 5, "other")` {
		t.Errorf("got %s", got)
	}
}

func TestMacroRulesRepetitions(t *testing.T) {
	extra := `
macro_rules! sum { ($($x:expr),*) => { 0 $(+ $x)* }; }
macro_rules! sum_semi { ($($x:expr);+ $(;)?) => { 0 $(+ $x)+ }; }
macro_rules! arr { ($($x:expr),+ $(,)?) => { [$($x),+] }; }
macro_rules! pairs { ($($k:ident => $v:expr),* $(,)?) => { [$(($k, $v)),*] }; }
macro_rules! nested { ($($name:ident: [$($v:expr),*]);*) => { ($( ($name, [$($v * 10),*]) ),*) }; }
macro_rules! opt { ($a:ident $(= $d:expr)?) => { ($a $(, $d)?) }; }
macro_rules! sepp { ($($x:ident)|+) => { [$(stringify!($x)),+] }; }
macro_rules! dblsep { ($($x:ident)::*) => { $($x)::* }; }
`
	cases := []struct{ src, want string }{
		{`sum!()`, `0`},
		{`sum!(1, 2, 3)`, `0 + 1 + 2 + 3`},
		{`sum_semi!(1; 2;)`, `0 + 1 + 2`},
		{`sum_semi!(4)`, `0 + 4`},
		{`arr!(1, 2,)`, `[1, 2]`},
		{`pairs!(a => 1, b => 2 + 3,)`, `[(a, 1), (b, (2 + 3))]`},
		{`nested!(x: [1, 2]; y: [])`, `((x, [1 * 10, 2 * 10]), (y, []))`},
		{`opt!(q)`, `(q)`},
		{`opt!(q = 7)`, `(q, 7)`},
		{`sepp!(a | b | c)`, `["a", "b", "c"]`},
		{`dblsep!(std::mem::swap)`, `std::mem::swap`},
	}
	for _, c := range cases {
		if got := expandExprSrc(t, extra, c.src); got != c.want {
			t.Errorf("%s: got %s want %s", c.src, got, c.want)
		}
	}
}

func TestMacroRulesRecursion(t *testing.T) {
	extra := `
macro_rules! count {
    () => { 0usize };
    ($head:tt $($tail:tt)*) => { 1usize + count!($($tail)*) };
}
macro_rules! rev {
    (@acc [$($acc:tt)*]) => { [$($acc),*] };
    (@acc [$($acc:tt)*] $x:tt $($rest:tt)*) => { rev!(@acc [$x $($acc)*] $($rest)*) };
    ($($x:tt)*) => { rev!(@acc [] $($x)*) };
}
macro_rules! max {
    ($x:expr) => { $x };
    ($x:expr, $($rest:expr),+) => { { let a = $x; let b = max!($($rest),+); if a > b { a } else { b } } };
}
`
	if got := expandExprSrc(t, extra, `count!(a b c)`); got != "1usize + (1usize + (1usize + 0usize))" {
		t.Errorf("count: %s", got)
	}
	if got := expandExprSrc(t, extra, `rev!(1 2 3)`); got != "[3, 2, 1]" {
		t.Errorf("rev: %s", got)
	}
	got := expandExprSrc(t, extra, `max!(1, 2, 3)`)
	contains(t, got, "let a = 1;", "let a = 2;", "let b = 3;")
	// long tt-muncher (within the default recursion limit)
	src := "count!(" + strings.Repeat("x ", 100) + ")"
	got = expandExprSrc(t, extra, src)
	if strings.Count(got, "1usize") != 100 {
		t.Errorf("count of 100: %d", strings.Count(got, "1usize"))
	}
	mustErr(t, `macro_rules! inf { () => { inf!() }; } fn main() { inf!(); }`, "recursion limit reached while expanding `inf!`")
}

func TestMacroRulesFragments(t *testing.T) {
	extra := `
macro_rules! mk_fn { ($v:vis $name:ident($($arg:ident: $t:ty),*) -> $ret:ty $body:block) => { $v fn $name($($arg: $t),*) -> $ret $body }; }
macro_rules! lit { ($l:literal) => { $l }; ($i:ident) => { "ident" }; }
macro_rules! lt { ($l:lifetime) => { &$l str }; }
macro_rules! pth { ($p:path) => { $p::new() }; }
macro_rules! pt { ($p:pat) => { match v { $p => 1, _ => 0 } }; }
macro_rules! ptp { ($($p:pat_param)|+) => { match v { $($p)|+ => 1, _ => 0 } }; }
macro_rules! st { ($($s:stmt);*) => { { $($s;)* } }; }
macro_rules! it { ($i:item) => { $i }; }
macro_rules! mt { ($(#[$m:meta])*) => { [$(stringify!($m)),*] }; }
macro_rules! idk { ($i:ident) => { stringify!($i) }; }
macro_rules! tys { ($t:ty) => { <$t>::default() }; }
macro_rules! te { ($t:tt) => { $t }; }
`
	c := expandSrc(t, extra+`
mk_fn!(pub add(a: i32, b: i32) -> i32 { a + b });
mk_fn!(sub(a: i32) -> Vec<i32> { vec![a] });
it!(struct Made { x: i32 });
fn main() {
    let a = lit!(-5);
    let b = lit!("s");
    let c = lit!(z);
    let d: lt!('static) = "";
    let e = pth!(std::collections::HashMap<i32, i32>);
    let f = pt!(Some(1) | None);
    let g = ptp!(1 | 2);
    let h = st!(let x = 1; x + 1);
    let i = mt!(#[derive(Debug)] #[allow(dead_code)]);
    let j = idk!(self);
    let k = tys!(Vec<u8>);
    let l = te!((1, 2));
}`)
	noMacros(t, c)
	s := crateString(c)
	contains(t, s,
		"fn add(a: i32, b: i32) -> i32 { a + b }",
		"fn sub(a: i32) -> Vec<i32> { ::std::vec::from_array([a]) }",
		"struct Made{x}",
		"let a = -5;",
		`let b = "s";`,
		`let c = "ident";`,
		`let d: &'static str = "";`,
		"let e = std::collections::HashMap::<i32, i32>::new();",
		"let f = match v { Some(1) | None => 1, _ => 0, };",
		"let g = match v { 1 | 2 => 1, _ => 0, };",
		"let h = { let x = 1; x + 1; };",
		`let i = ["derive(Debug)", "allow(dead_code)"];`,
		`let j = "self";`,
		"let k = <Vec<u8>>::default();",
		"let l = (1, 2);",
	)
	if it := findItem(c.Items, "add"); it == nil || it.Vis != ast.VisPub {
		t.Errorf("visibility not transcribed")
	}
	mustErr(t, `macro_rules! m { ($i:ident) => {}; } fn main() { m!(_); }`, "no rules of macro `m` matched")
}

func TestMacroRulesPositions(t *testing.T) {
	c := expandSrc(t, `
macro_rules! ty { () => { Vec<i32> }; }
macro_rules! pat { ($x:ident) => { Some($x) }; }
macro_rules! items { ($($n:ident),*) => { $(const $n: i32 = 1;)* }; }
macro_rules! stmts { () => { let a = 1; let b = 2; }; }
macro_rules! tail { () => { a + b }; }
macro_rules! nothing { () => {}; }
items!(A, B);
fn main() {
    let v: ty!() = Vec::new();
    if let pat!(q) = o {}
    stmts!();
    nothing!();
    tail!()
}
fn g() -> i32 { stmts! { } tail!{} }
fn h() { tail!(); }
`)
	noMacros(t, c)
	s := crateString(c)
	contains(t, s, "const A: i32 = 1;", "const B: i32 = 1;",
		"let v: Vec<i32> = Vec::new();",
		"if let Some(q) = o {}",
		"let a = 1; let b = 2; a + b }",
		"fn g() -> i32 { let a = 1; let b = 2; a + b }",
		"fn h() { a + b; }",
	)
	main := findItem(c.Items, "main").Kind.(*ast.FnItem).Body
	last := main.Stmts[len(main.Stmts)-1].(*ast.ExprStmt)
	if last.Semi {
		t.Errorf("tail expression should not have a semicolon")
	}
}

func TestMacroRulesCrateAndStd(t *testing.T) {
	e := New()
	std := parseCrate(t, "std.rs", `
#[macro_export]
macro_rules! my_vec { ($($x:expr),*) => { $crate::vec::Vec::from([$($x),*]) }; }
macro_rules! helper { () => { $crate::thing() }; }
pub fn thing() -> i32 { helper!() }
`)
	if err := e.ExpandCrate(std, "std"); err != nil {
		t.Fatal(err)
	}
	if got := fnBody(t, std, "thing"); got != "{ ::std::thing() }" {
		t.Errorf("std $crate: %s", got)
	}
	user := parseCrate(t, "main.rs", `
macro_rules! local { () => { $crate::f() }; }
fn main() { let v = my_vec![1, 2]; let w = local!(); let h = helper!(); }
`)
	if err := e.ExpandCrate(user, "main"); err != nil {
		t.Fatal(err)
	}
	b := fnBody(t, user, "main")
	contains(t, b, "let v = ::std::vec::Vec::from([1, 2]);", "let w = crate::f();", "let h = ::std::thing();")
}

func TestMacroRulesOrderAndScopes(t *testing.T) {
	c := expandSrc(t, `
fn main() { let x = later!(); }
fn f() {
    macro_rules! local { ($a:expr) => { $a + 100 }; }
    let y = local!(1);
    {
        let z = local!(2);
    }
}
mod m {
    macro_rules! in_mod { () => { 7 }; }
    pub fn g() -> i32 { in_mod!() }
}
macro_rules! later { () => { 42 }; }
`)
	s := crateString(c)
	contains(t, s, "let x = 42;", "let y = 1 + 100;", "let z = 2 + 100;", "pub fn g() -> i32 { 7 }"[4:])
	notContains(t, s, "MacroRulesItem")
}

func TestMacroGeneratingMacro(t *testing.T) {
	got := expandExprSrc(t, `
macro_rules! make { ($name:ident, $v:expr) => { macro_rules! $name { () => { $v }; } }; }
make!(seven, 7);
macro_rules! wrap { ($($t:tt)*) => { vec![$($t)*] }; }
`, `(seven!(), wrap!(1, 2))`)
	contains(t, got, "(7, ::std::vec::from_array([1, 2]))")
}

func TestMacroRulesSiteLocation(t *testing.T) {
	src := "macro_rules! here {\n    () => { line!() };\n}\nmacro_rules! boom { () => { panic!() }; }\nfn main() {\n    let a = here!();\n    boom!();\n}"
	c := expandSrc(t, src)
	b := fnBody(t, c, "main")
	contains(t, b, "let a = 6u32;", `::std::rt::panic_str("explicit panic", "test.rs", 7, 5);`)
}

func TestMacroRulesErrors(t *testing.T) {
	mustErr(t, `macro_rules! m { ($a:expr) => { $a }; } fn main() { m!(1, 2); }`, "no rules of macro `m` matched")
	mustErr(t, `macro_rules! m { ($a) => { $a }; }`, "missing fragment specifier")
	mustErr(t, `macro_rules! m { ($a:foo) => { $a }; }`, "invalid fragment specifier `foo`")
	mustErr(t, `macro_rules! m { ($($a:ident)*) => { $a }; } fn main() { m!(x); }`, "still repeating")
	mustErr(t, `macro_rules! m { ($($a:ident)*; $($b:ident)*) => { $(($a, $b))* }; } fn main() { m!(x y; z); }`, "repeats 2 times, but `b` repeats 1 times")
	mustErr(t, `macro_rules! m { () => { $(x)* }; } fn main() { m!(); }`, "no syntax variables")
	mustErr(t, `macro_rules! m { () => { 1 2 }; } fn main() { let x = m!(); }`, "macro expansion ignores token `2`")
}

// ---------------------------------------------------------------------------
// Derives

func TestDeriveStruct(t *testing.T) {
	c := expandSrc(t, `
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
struct P<T: Copy, const N: usize> where T: Sized { x: T, r#type: [T; N] }
`)
	s := crateString(c)
	contains(t, s,
		`impl<T: Copy + ::std::fmt::Debug, const N: usize> ::std::fmt::Debug for P<T, N> where T: Sized { fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result { f.debug_struct("P").field("x", &self.x).field("type", &self.r#type).finish() } }`,
		`impl<T: Copy + ::std::clone::Clone, const N: usize> ::std::clone::Clone for P<T, N> where T: Sized { fn clone(&self) -> Self { P { x: ::std::clone::Clone::clone(&self.x), r#type: ::std::clone::Clone::clone(&self.r#type) } } }`,
		`impl<T: Copy + ::std::marker::Copy, const N: usize> ::std::marker::Copy for P<T, N> where T: Sized {  }`,
		`fn eq(&self, other: &Self) -> bool { self.x == other.x && self.r#type == other.r#type }`,
		`impl<T: Copy + ::std::cmp::Eq, const N: usize> ::std::cmp::Eq for P<T, N>`,
		`fn partial_cmp(&self, other: &Self) -> ::std::option::Option<::std::cmp::Ordering> { match ::std::cmp::PartialOrd::partial_cmp(&self.x, &other.x) { ::std::option::Option::Some(::std::cmp::Ordering::Equal) => match ::std::cmp::PartialOrd::partial_cmp(&self.r#type, &other.r#type) { ::std::option::Option::Some(::std::cmp::Ordering::Equal) => ::std::option::Option::Some(::std::cmp::Ordering::Equal), cmp => cmp, }, cmp => cmp, } }`,
		`fn cmp(&self, other: &Self) -> ::std::cmp::Ordering { match ::std::cmp::Ord::cmp(&self.x, &other.x) { ::std::cmp::Ordering::Equal => match ::std::cmp::Ord::cmp(&self.r#type, &other.r#type) { ::std::cmp::Ordering::Equal => ::std::cmp::Ordering::Equal, cmp => cmp, }, cmp => cmp, } }`,
		`fn hash<__H: ::std::hash::Hasher>(&self, state: &mut __H) { ::std::hash::Hash::hash(&self.x, state); ::std::hash::Hash::hash(&self.r#type, state); }`,
		`fn default() -> Self { P { x: ::std::default::Default::default(), r#type: ::std::default::Default::default() } }`,
	)
	// derive attributes are consumed
	p := findItem(c.Items, "P")
	if hasAttr(p.Attrs, "derive") {
		t.Errorf("derive attribute not removed")
	}
	// impls follow the type directly
	if len(c.Items) != 10 {
		t.Errorf("expected 10 items, got %d", len(c.Items))
	}
}

func TestDeriveTupleAndUnit(t *testing.T) {
	c := expandSrc(t, `
#[derive(Debug, Clone, PartialEq, Default, std::hash::Hash)]
struct T(i32, String);
#[derive(Debug, Clone, PartialEq, PartialOrd, Default)]
struct U;
`)
	s := crateString(c)
	contains(t, s,
		`f.debug_tuple("T").field(&self.0).field(&self.1).finish()`,
		`T(::std::clone::Clone::clone(&self.0), ::std::clone::Clone::clone(&self.1))`,
		`self.0 == other.0 && self.1 == other.1`,
		`T(::std::default::Default::default(), ::std::default::Default::default())`,
		`::std::hash::Hash::hash(&self.0, state); ::std::hash::Hash::hash(&self.1, state);`,
		`f.write_str("U")`,
		`fn clone(&self) -> Self { U }`,
		`fn eq(&self, other: &Self) -> bool { true }`,
		`fn partial_cmp(&self, other: &Self) -> ::std::option::Option<::std::cmp::Ordering> { ::std::option::Option::Some(::std::cmp::Ordering::Equal) }`,
		`fn default() -> Self { U }`,
	)
}

func TestDeriveEnum(t *testing.T) {
	c := expandSrc(t, `
#[derive(Debug, Clone, PartialEq, PartialOrd, Ord, Hash, Default)]
enum E<'a, T> { #[default] A, B(T, &'a str), C { x: T, f: i32 } }
#[derive(Debug, Clone, PartialEq, Hash, PartialOrd)]
enum Empty {}
#[derive(PartialEq, Ord, Hash)]
enum Units { X, Y }
`)
	s := crateString(c)
	contains(t, s,
		`impl<'a, T: ::std::fmt::Debug> ::std::fmt::Debug for E<'a, T>`,
		`match self { Self::A => f.write_str("A"), Self::B(__self_0, __self_1) => f.debug_tuple("B").field(__self_0).field(__self_1).finish(), Self::C { x: __self_0, f: __self_1 } => f.debug_struct("C").field("x", __self_0).field("f", __self_1).finish(), }`,
		`match self { Self::A => Self::A, Self::B(__self_0, __self_1) => Self::B(::std::clone::Clone::clone(__self_0), ::std::clone::Clone::clone(__self_1)), Self::C { x: __self_0, f: __self_1 } => Self::C { x: ::std::clone::Clone::clone(__self_0), f: ::std::clone::Clone::clone(__self_1) }, }`,
		`let __self_discr = ::std::intrinsics::discriminant_value(self); let __arg1_discr = ::std::intrinsics::discriminant_value(other); __self_discr == __arg1_discr && match (self, other) { (Self::B(__self_0, __self_1), Self::B(__arg1_0, __arg1_1)) => *__self_0 == *__arg1_0 && *__self_1 == *__arg1_1, (Self::C { x: __self_0, f: __self_1 }, Self::C { x: __arg1_0, f: __arg1_1 }) => *__self_0 == *__arg1_0 && *__self_1 == *__arg1_1, _ => true, }`,
		`if __self_discr != __arg1_discr { return ::std::cmp::PartialOrd::partial_cmp(&__self_discr, &__arg1_discr); }; match (self, other) { (Self::B(__self_0, __self_1), Self::B(__arg1_0, __arg1_1)) => match ::std::cmp::PartialOrd::partial_cmp(__self_0, __arg1_0) {`,
		`_ => ::std::option::Option::Some(::std::cmp::Ordering::Equal), }`,
		`if __self_discr != __arg1_discr { return ::std::cmp::Ord::cmp(&__self_discr, &__arg1_discr); }`,
		`::std::hash::Hash::hash(&::std::intrinsics::discriminant_value(self), state); match self { Self::B(__self_0, __self_1) => { ::std::hash::Hash::hash(__self_0, state); ::std::hash::Hash::hash(__self_1, state); }, Self::C { x: __self_0, f: __self_1 } => { ::std::hash::Hash::hash(__self_0, state); ::std::hash::Hash::hash(__self_1, state); }, _ => {}, }`,
		`fn default() -> Self { Self::A }`,
		// empty enum
		`::std::fmt::Debug for Empty { fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result { match *self { } } }`,
		`fn clone(&self) -> Self { match *self { } }`,
		// fieldless enums
		`fn eq(&self, other: &Self) -> bool { let __self_discr = ::std::intrinsics::discriminant_value(self); let __arg1_discr = ::std::intrinsics::discriminant_value(other); __self_discr == __arg1_discr }`,
		`fn cmp(&self, other: &Self) -> ::std::cmp::Ordering { let __self_discr = ::std::intrinsics::discriminant_value(self); let __arg1_discr = ::std::intrinsics::discriminant_value(other); ::std::cmp::Ord::cmp(&__self_discr, &__arg1_discr) }`,
	)
}

func TestDeriveNestedAndErrors(t *testing.T) {
	c := expandSrc(t, `
fn main() {
    #[derive(Clone)]
    struct Local { a: i32 }
    let x = 1;
}
macro_rules! mk { ($n:ident) => { #[derive(Debug)] struct $n; }; }
mk!(Gen);
`)
	s := crateString(c)
	contains(t, s, "/* item Local */ /* item  */ let x = 1;", `::std::fmt::Debug for Gen { fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result { f.write_str("Gen") } }`)
	mustErr(t, `#[derive(Frobnicate)] struct S;`, "cannot find derive macro `Frobnicate` in this scope")
	mustErr(t, `#[derive(Default)] enum E { A, B }`, "no default declared")
	mustErr(t, `#[derive(Debug)] fn f() {}`, "`derive` may only be applied")
}

// ---------------------------------------------------------------------------
// cfg

func TestCfg(t *testing.T) {
	c := expandSrc(t, `
#[cfg(test)]
mod tests { #[test] fn t() {} }
#[test]
fn a_test() {}
#[cfg(not(test))]
fn kept() {}
#[cfg(windows)]
fn win() {}
#[cfg(all(unix, target_os = "linux", target_pointer_width = "64", not(feature = "x")))]
fn linux() {}
#[cfg(any(windows, target_arch = "x86_64"))]
fn any_ok() {}
#[cfg(any())]
fn any_empty() {}
#[cfg(all())]
fn all_empty() {}
#[cfg_attr(debug_assertions, derive(Clone))]
struct S { a: i32, #[cfg(windows)] b: i32 }
#[cfg_attr(test, derive(Debug))]
struct S2;
#[cfg_attr(not(test), cfg(test))]
fn gone() {}
enum E { A, #[cfg(windows)] B, C }
fn main() {
    #[cfg(windows)]
    let x = 1;
    #[cfg(unix)]
    let y = 2;
    #[cfg(windows)]
    println!("w");
    let v = match y { #[cfg(windows)] 1 => 1, _ => 2 };
    f(1, #[cfg(windows)] 2, 3);
    let t = (1, #[cfg(windows)] 2);
    #[cfg(windows)]
    { win(); }
    #[cfg(debug_assertions)]
    { dbg(); }
}
`)
	s := crateString(c)
	notContains(t, s, "mod tests", "a_test", "fn win", "any_empty", "fn gone", "let x = 1", `"w"`, "1 => 1", "win();", "Debug for S2")
	contains(t, s, "fn kept", "fn linux", "fn any_ok", "fn all_empty", "struct S{a}", "Clone for S", "enum E{A,C}",
		"let y = 2;", "let v = match y { _ => 2, };", "f(1, 3);", "let t = (1,);", "{ dbg(); }")
	mustErr(t, `#[cfg(foo(bar))] fn f() {}`, "invalid predicate `foo`")
	mustErr(t, `#[cfg(a, b)] fn f() {}`, "single predicate")
}

func TestEvalCfg(t *testing.T) {
	cases := map[string]bool{
		"test": false, "debug_assertions": true, "unix": true, "windows": false,
		`target_os = "linux"`: true, `target_os = "macos"`: false, `target_pointer_width = "64"`: true,
		`target_pointer_width = "32"`: false, `target_arch = "x86_64"`: true, `feature = "std"`: false,
		"not(test)": true, "all(unix, debug_assertions)": true, "all(unix, test)": false,
		"any(test, windows)": false, "any(test, unix)": true, "not(any(windows, all(test)))": true,
		"unknown_flag": false,
	}
	for src, want := range cases {
		tts, err := parser.TokenizeToTrees(0, src)
		if err != nil {
			t.Fatal(err)
		}
		got, err := EvalCfg(tts, lexer.Span{})
		if err != nil {
			t.Errorf("%s: %v", src, err)
			continue
		}
		if got != want {
			t.Errorf("%s: got %v want %v", src, got, want)
		}
	}
}

// ---------------------------------------------------------------------------
// Walk coverage

func TestExpansionEverywhere(t *testing.T) {
	c := expandSrc(t, `
macro_rules! one { () => { 1 }; }
macro_rules! t { () => { i32 }; }
const C: t!() = one!();
static S: [t!(); one!()] = [one!()];
struct W { f: t!() }
trait Tr { fn m(&self) -> i32 { one!() } const K: i32 = one!(); }
impl W { fn new() -> Self { W { f: one!() } } }
fn main() {
    let cl = |x: t!()| -> t!() { x + one!() };
    let arr = [one!(); 3];
    let r = one!()..one!();
    for i in vec![one!()] { while one!() > 0 { loop { break one!(); } } }
    let s = W { f: one!() }.f;
    let m = match one!() { x if x == one!() => one!(), _ => one!() };
    let c = (one!() as i64, -one!(), !one!(), &one!(), one!().abs(), foo(one!())[one!()]);
    if let Some(x) = Some(one!()) { x } else { one!() };
    let opt = Some(1)?;
    unsafe { one!() };
    return one!();
}
`)
	noMacros(t, c)
}

func TestStmtMacroBuiltinSemicolon(t *testing.T) {
	c := expandSrc(t, `fn main() { println!("a") } fn f() -> Vec<i32> { vec![1] }`)
	m := findItem(c.Items, "main").Kind.(*ast.FnItem).Body
	if es, ok := m.Stmts[0].(*ast.ExprStmt); !ok || es.Semi {
		t.Errorf("tail println! should be an ExprStmt without semicolon")
	}
	f := findItem(c.Items, "f").Kind.(*ast.FnItem).Body
	if es, ok := f.Stmts[0].(*ast.ExprStmt); !ok || es.Semi {
		t.Errorf("tail vec! should be an ExprStmt without semicolon")
	}
}

func TestMacroInImplAndTrait(t *testing.T) {
	c := expandSrc(t, `
macro_rules! getter { ($name:ident: $t:ty) => { pub fn $name(&self) -> $t { self.$name } }; }
macro_rules! assoc { () => { type Out = i32; const N: usize = 3; fn req(&self); }; }
struct S { a: i32 }
impl S { getter!(a: i32); }
trait T { assoc!(); }
`)
	s := crateString(c)
	contains(t, s, "impl S { fn a(&self) -> i32 { self.a } }", "<*ast.TraitItem T>")
	tr := findItem(c.Items, "T").Kind.(*ast.TraitItem)
	if len(tr.Items) != 3 {
		t.Errorf("trait items: %d", len(tr.Items))
	}
}

func TestFragmentForwarding(t *testing.T) {
	extra := `
macro_rules! p { ($fmt:expr, $($a:expr),*) => { format!($fmt, $($a),*) }; }
macro_rules! show { ($($e:expr),*) => { [$(stringify!($e)),*] }; }
macro_rules! chk { ($e:expr) => { assert!($e) }; }
macro_rules! mk { ($p:path) => { $p::new() }; }
macro_rules! fwd { ($e:expr) => { inner!($e) }; }
macro_rules! inner { ($x:expr) => { $x * 10 }; }
`
	got := expandExprSrc(t, extra, `p!(concat!("{}", "-"), a + 1)`)
	contains(t, got, `match (&(a + 1),)`, `__fmt.write_str("-")`)
	if got := expandExprSrc(t, extra, `show!(x * 2 + 1, vec![x; 3])`); got != `["x * 2 + 1", "vec![x; 3]"]` {
		t.Errorf("show!: %s", got)
	}
	contains(t, expandExprSrc(t, extra, `chk!(a == b)`), `"assertion failed: a == b"`)
	if got := expandExprSrc(t, extra, `mk!(Vec<i32>)`); got != "Vec::<i32>::new()" {
		t.Errorf("mk!: %s", got)
	}
	if got := expandExprSrc(t, extra, `fwd!(1 + 2)`); got != "(1 + 2) * 10" {
		t.Errorf("fwd!: %s", got)
	}
}

func TestErrorFormat(t *testing.T) {
	_, err := expandSrcErr(t, "fn main() {\n  let x = nope!();\n}")
	if err == nil || err.Error() != "test.rs:2:11: error: cannot find macro `nope` in this scope" {
		t.Errorf("got %v", err)
	}
	// parse errors inside an expansion are reported in the same format
	_, err = expandSrcErr(t, "macro_rules! bad { () => { 1 + }; }\nfn main() { let x = bad!(); }")
	if err == nil || !strings.Contains(err.Error(), ": error: ") || !strings.Contains(err.Error(), "in expansion of `bad!`") {
		t.Errorf("got %v", err)
	}
}

// TestCorpus expands every run-pass test program (files the parser rejects
// are skipped) and checks that no macro invocation is left.
func TestCorpus(t *testing.T) {
	files, _ := filepath.Glob("../../tests/run-pass/*.rs")
	more, _ := filepath.Glob("../../tests/run-pass/*/*.rs")
	files = append(files, more...)
	if len(files) == 0 {
		t.Skip("no test programs")
	}
	// std defines some macros (thread_local!, ...): expand it first.
	e := New()
	stdCrate, err := stdlib.LoadCrate(func(name, src string) int { return lexer.AddFile(name, src) })
	if err != nil {
		t.Fatal(err)
	}
	if err := e.ExpandCrate(stdCrate, "std"); err != nil {
		t.Fatal(err)
	}
	for _, f := range files {
		src, err := os.ReadFile(f)
		if err != nil {
			t.Fatal(err)
		}
		id := lexer.AddFile(f, string(src))
		c, err := parser.ParseCrate(id, string(src))
		if err != nil {
			continue
		}
		if err := e.ExpandCrate(c, "main"); err != nil {
			t.Errorf("%s: %v", f, err)
			continue
		}
		noMacros(t, c)
	}
}
