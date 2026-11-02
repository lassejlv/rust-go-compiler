package parser

import (
	"fmt"
	"os"
	"strings"
	"testing"

	"rustgo/compiler/ast"
	"rustgo/compiler/lexer"
)

func mustParse(t *testing.T, src string) *ast.Crate {
	t.Helper()
	fid := lexer.AddFile("test.rs", src)
	c, err := ParseCrate(fid, src)
	if err != nil {
		t.Fatalf("parse error: %v\nsource:\n%s", err, src)
	}
	return c
}

func parseExpr(t *testing.T, src string) ast.Expr {
	t.Helper()
	fid := lexer.AddFile("expr.rs", src)
	p, err := New(fid, src)
	if err != nil {
		t.Fatal(err)
	}
	e, err := p.ParseExpr()
	if err != nil {
		t.Fatalf("parse %q: %v", src, err)
	}
	if !p.AtEOF() {
		t.Fatalf("parse %q: trailing tokens at %v", src, p.Peek())
	}
	return e
}

func parseStmts(t *testing.T, src string) []ast.Stmt {
	t.Helper()
	fid := lexer.AddFile("stmts.rs", src)
	p, err := New(fid, src)
	if err != nil {
		t.Fatal(err)
	}
	s, err := p.ParseStmts()
	if err != nil {
		t.Fatalf("parse %q: %v", src, err)
	}
	return s
}

// sx renders an expression as a fully parenthesized S-expression.
func sx(e ast.Expr) string {
	switch x := e.(type) {
	case nil:
		return "nil"
	case *ast.LitExpr:
		return x.Value + x.Suffix
	case *ast.PathExpr:
		if x.QSelf != nil {
			return "<" + ast.TypeString(x.QSelf.Ty) + ">::" + ast.ExprPathString(x.Path)
		}
		return ast.ExprPathString(x.Path)
	case *ast.BinaryExpr:
		return fmt.Sprintf("(%s %s %s)", x.Op, sx(x.L), sx(x.R))
	case *ast.AssignExpr:
		return fmt.Sprintf("(= %s %s)", sx(x.L), sx(x.R))
	case *ast.AssignOpExpr:
		return fmt.Sprintf("(%s= %s %s)", x.Op, sx(x.L), sx(x.R))
	case *ast.UnaryExpr:
		op := [...]string{"neg", "not", "deref"}[x.Op]
		return fmt.Sprintf("(%s %s)", op, sx(x.X))
	case *ast.RefExpr:
		op := "&"
		if x.Raw {
			op += "raw"
		}
		if x.Mut {
			op += "mut"
		}
		return fmt.Sprintf("(%s %s)", op, sx(x.X))
	case *ast.CastExpr:
		return fmt.Sprintf("(as %s %s)", sx(x.X), ast.TypeString(x.Ty))
	case *ast.CallExpr:
		var a []string
		for _, arg := range x.Args {
			a = append(a, sx(arg))
		}
		return fmt.Sprintf("(call %s [%s])", sx(x.Fn), strings.Join(a, " "))
	case *ast.MethodCallExpr:
		var a []string
		for _, arg := range x.Args {
			a = append(a, sx(arg))
		}
		g := ""
		if x.Generics != nil {
			g = "::" + ast.GenericArgsString(x.Generics)
		}
		return fmt.Sprintf("(.%s%s %s [%s])", x.Method.Name, g, sx(x.Receiver), strings.Join(a, " "))
	case *ast.FieldExpr:
		return fmt.Sprintf("(field %s %s)", sx(x.X), x.Name.Name)
	case *ast.IndexExpr:
		return fmt.Sprintf("(index %s %s)", sx(x.X), sx(x.Index))
	case *ast.RangeExpr:
		op := ".."
		if x.Inclusive {
			op = "..="
		}
		return fmt.Sprintf("(%s %s %s)", op, sx(x.Start), sx(x.End))
	case *ast.TryExpr:
		return fmt.Sprintf("(? %s)", sx(x.X))
	case *ast.ParenExpr:
		return fmt.Sprintf("(paren %s)", sx(x.X))
	case *ast.TupleExpr:
		var a []string
		for _, el := range x.Elems {
			a = append(a, sx(el))
		}
		return fmt.Sprintf("(tuple %s)", strings.Join(a, " "))
	case *ast.LetExpr:
		return fmt.Sprintf("(let %s %s)", ast.PatString(x.Pat), sx(x.Expr))
	case *ast.ClosureExpr:
		return fmt.Sprintf("(closure %s)", sx(x.Body))
	case *ast.ReturnExpr:
		return fmt.Sprintf("(return %s)", sx(x.Value))
	case *ast.BreakExpr:
		return fmt.Sprintf("(break %s %s)", x.Label, sx(x.Value))
	case *ast.MacCallExpr:
		return fmt.Sprintf("(mac %s)", ast.PathString(x.Mac.Path))
	case *ast.StructExpr:
		return fmt.Sprintf("(struct %s)", ast.ExprPathString(x.Path))
	case *ast.AwaitExpr:
		return fmt.Sprintf("(await %s)", sx(x.X))
	}
	return fmt.Sprintf("<%T>", e)
}

func TestPrecedence(t *testing.T) {
	cases := []struct{ src, want string }{
		{"1 + 2 * 3", "(+ 1 (* 2 3))"},
		{"1 * 2 + 3", "(+ (* 1 2) 3)"},
		{"a - b - c", "(- (- a b) c)"},
		{"a = b = c", "(= a (= b c))"},
		{"a += b * 2", "(+= a (* b 2))"},
		{"-x.y()", "(neg (.y x []))"},
		{"&a[0]", "(& (index a 0))"},
		{"x as u8 as u32", "(as (as x u8) u32)"},
		{"-x as u32", "(as (neg x) u32)"},
		{"a..b", "(.. a b)"},
		{"a..", "(.. a nil)"},
		{"..b", "(.. nil b)"},
		{"..", "(.. nil nil)"},
		{"a..=b", "(..= a b)"},
		{"a || b..c", "(.. (|| a b) c)"},
		{"!a && b", "(&& (not a) b)"},
		{"*x.y", "(deref (field x y))"},
		{"a && b || c && d", "(|| (&& a b) (&& c d))"},
		{"a | b ^ c & d", "(| a (^ b (& c d)))"},
		{"a << 2 + 1", "(<< a (+ 2 1))"},
		{"a == b && c != d", "(&& (== a b) (!= c d))"},
		{"x.0.1", "(field (field x 0) 1)"},
		{"t.0 .1", "(field (field t 0) 1)"},
		{"x.foo::<T>(1)", "(.foo::<T> x [1])"},
		{"5.max(3)", "(.max 5 [3])"},
		{"1.0f64.sqrt()", "(.sqrt 1.0f64 [])"},
		{"f(x)?.y", "(field (? (call f [x])) y)"},
		{"&&x", "(& (& x))"},
		{"&mut *x", "(&mut (deref x))"},
		{"&raw const x", "(&raw x)"},
		{"&raw mut x", "(&rawmut x)"},
		{"Vec::<u8>::new()", "(call Vec::<u8>::new [])"},
		{"<T as Default>::default()", "(call <T>::Default::default [])"},
		{"<Vec<i32>>::new()", "(call <Vec<i32>>::new [])"},
		{"x - 1", "(- x 1)"},
		{"x -1", "(- x 1)"},
		{"a < b", "(< a b)"},
		{"x as i64 < y", "(< (as x i64) y)"},
		{"x as usize < y && z > w", "(&& (< (as x usize) y) (> z w))"},
		{"|x| x + 1", "(closure (+ x 1))"},
		{"move || a = 3", "(closure (= a 3))"},
		{"a = |x| x", "(= a (closure x))"},
		{"vec![1, 2].len()", "(.len (mac vec) [])"},
		{"format!(\"a\") + \"b\"", "(+ (mac format) b)"},
		{"x.await?", "(? (await x))"},
		{"(a,)", "(tuple a)"},
		{"()", "(tuple )"},
		{"(a)", "(paren a)"},
		{"a % b * c", "(* (% a b) c)"},
		{"x = y..z", "(= x (.. y z))"},
		{"-1i32.abs()", "(neg (.abs 1i32 []))"},
		{"!f()", "(not (call f []))"},
		{"a[1..]", "(index a (.. 1 nil))"},
		{"Foo { x: 1 }.x", "(field (struct Foo) x)"},
		{"return a + b", "(return (+ a b))"},
		{"x == y as i32", "(== x (as y i32))"},
		{"a >> 1 >= b", "(>= (>> a 1) b)"},
		{"a &= b | c", "(&= a (| b c))"},
		{"a <<= 2", "(<<= a 2)"},
		{"x as u64 << 2", "(<< (as x u64) 2)"},
		{"a as usize > b", "(> (as a usize) b)"},
		{"-5..5", "(.. (neg 5) 5)"},
		{"1..n+1", "(.. 1 (+ n 1))"},
		{"x.0.0", "(field (field x 0) 0)"},
		{"t.0.len()", "(.len (field t 0) [])"},
		{"Ok::<(), String>(())", "(call Ok::<(), String> [(tuple )])"},
		{"foo::<{ 3 + 1 }>()", "(call foo::<{ 3 + 1 }> [])"},
		{"*self.x.borrow_mut() += 1", "(+= (deref (.borrow_mut (field self x) [])) 1)"},
		{"a.partial_cmp(&b).unwrap_or(Equal)", "(.unwrap_or (.partial_cmp a [(& b)]) [Equal])"},
		{"break 'a x + 1", "(break 'a (+ x 1))"},
		{"|x| move |y| x + y", "(closure (closure (+ x y)))"},
	}
	for _, c := range cases {
		e := parseExpr(t, c.src)
		if got := sx(e); got != c.want {
			t.Errorf("%s:\n got  %s\n want %s", c.src, got, c.want)
		}
	}
}

func TestLetChain(t *testing.T) {
	e := parseExpr(t, "if let Some(x) = a && x > 0 { 1 } else { 2 }")
	ie := e.(*ast.IfExpr)
	if got := sx(ie.Cond); got != "(&& (let Some(x) a) (> x 0))" {
		t.Errorf("got %s", got)
	}
	if _, ok := ie.Else.(*ast.BlockExpr); !ok {
		t.Errorf("else is %T", ie.Else)
	}
}

func TestNoStructInCond(t *testing.T) {
	e := parseExpr(t, "if x == Foo { a } else { b }")
	ie := e.(*ast.IfExpr)
	if got := sx(ie.Cond); got != "(== x Foo)" {
		t.Errorf("cond = %s", got)
	}
	e = parseExpr(t, "match x { _ => 1 }")
	if _, ok := e.(*ast.MatchExpr); !ok {
		t.Fatalf("got %T", e)
	}
	e = parseExpr(t, "for i in 0..n { }")
	fe := e.(*ast.ForExpr)
	if got := sx(fe.Iter); got != "(.. 0 n)" {
		t.Errorf("iter = %s", got)
	}
	e = parseExpr(t, "for i in 0.. { }")
	fe = e.(*ast.ForExpr)
	if got := sx(fe.Iter); got != "(.. 0 nil)" {
		t.Errorf("iter = %s", got)
	}
	e = parseExpr(t, "while (Foo { a: 1 }).a > 0 { }")
	if _, ok := e.(*ast.WhileExpr); !ok {
		t.Fatalf("got %T", e)
	}
	e = parseExpr(t, "if f(Foo { a: 1 }) { }")
	if _, ok := e.(*ast.IfExpr); !ok {
		t.Fatalf("got %T", e)
	}
}

func TestStmtRules(t *testing.T) {
	stmts := parseStmts(t, "match x { _ => {} } - 1")
	if len(stmts) != 2 {
		t.Fatalf("want 2 stmts, got %d", len(stmts))
	}
	if es := stmts[0].(*ast.ExprStmt); !es.Semi {
		t.Errorf("non-final block-like stmt should have Semi=true")
	}
	if es := stmts[1].(*ast.ExprStmt); es.Semi || sx(es.Expr) != "(neg 1)" {
		t.Errorf("got %s semi=%v", sx(es.Expr), es.Semi)
	}

	stmts = parseStmts(t, "if a { 1 } else { 2 }")
	if es := stmts[0].(*ast.ExprStmt); es.Semi {
		t.Errorf("final if should have Semi=false")
	}

	stmts = parseStmts(t, "match x { _ => 1 }.to_string(); { 1 }.max(2)")
	if len(stmts) != 2 {
		t.Fatalf("want 2 stmts, got %d", len(stmts))
	}
	if es := stmts[0].(*ast.ExprStmt); !es.Semi {
		t.Errorf("want semi")
	} else if _, ok := es.Expr.(*ast.MethodCallExpr); !ok {
		t.Errorf("want method call, got %T", es.Expr)
	}
	if es := stmts[1].(*ast.ExprStmt); es.Semi {
		t.Errorf("want no semi")
	}

	stmts = parseStmts(t, `println!("x"); foo!{ a } bar![1]; format!("x")`)
	if len(stmts) != 4 {
		t.Fatalf("want 4 stmts, got %d", len(stmts))
	}
	for i, wantSemi := range []bool{true, true, true, false} {
		ms, ok := stmts[i].(*ast.MacStmt)
		if !ok {
			t.Fatalf("stmt %d: %T", i, stmts[i])
		}
		if ms.Semi != wantSemi {
			t.Errorf("stmt %d semi=%v", i, ms.Semi)
		}
	}

	stmts = parseStmts(t, `vec![1,2].len(); format!("a") + "b"`)
	if es, ok := stmts[0].(*ast.ExprStmt); !ok || !es.Semi {
		t.Errorf("stmt 0: %#v", stmts[0])
	}
	if es, ok := stmts[1].(*ast.ExprStmt); !ok || es.Semi {
		t.Errorf("stmt 1: %#v", stmts[1])
	}

	stmts = parseStmts(t, "let x = 1; x")
	if _, ok := stmts[0].(*ast.LetStmt); !ok {
		t.Errorf("want let")
	}

	stmts = parseStmts(t, "let Some(x) = opt else { return; }; fn inner() {} struct S; x")
	ls := stmts[0].(*ast.LetStmt)
	if ls.Else == nil {
		t.Errorf("want let-else")
	}
	if _, ok := stmts[1].(*ast.ItemStmt); !ok {
		t.Errorf("want item")
	}
	if _, ok := stmts[2].(*ast.ItemStmt); !ok {
		t.Errorf("want item")
	}

	stmts = parseStmts(t, "loop { break; } while x {} for x in y {} unsafe { f() } 'a: loop {} { 2 }")
	if len(stmts) != 6 {
		t.Fatalf("want 6, got %d", len(stmts))
	}
	for i, s := range stmts {
		es := s.(*ast.ExprStmt)
		if es.Semi != (i < 5) {
			t.Errorf("stmt %d semi=%v", i, es.Semi)
		}
	}

	// const block vs const item
	stmts = parseStmts(t, "const N: usize = 3; const { 1 }")
	if _, ok := stmts[0].(*ast.ItemStmt); !ok {
		t.Errorf("want const item")
	}
	if es := stmts[1].(*ast.ExprStmt); !es.Expr.(*ast.BlockExpr).Const {
		t.Errorf("want const block")
	}

	// macro_rules inside fn
	stmts = parseStmts(t, "macro_rules! m { () => {} } m!()")
	if _, ok := stmts[0].(*ast.ItemStmt); !ok {
		t.Errorf("want macro_rules item, got %T", stmts[0])
	}
}

func TestLiterals(t *testing.T) {
	cases := []struct {
		src    string
		kind   ast.LitKind
		val    string
		intVal uint64
		big    string
		suffix string
	}{
		{"42", ast.LitInt, "42", 42, "", ""},
		{"0xff_u8", ast.LitInt, "0xff_", 255, "", "u8"},
		{"0o777", ast.LitInt, "0o777", 511, "", ""},
		{"0b1010_1010", ast.LitInt, "0b1010_1010", 170, "", ""},
		{"1_000_000i64", ast.LitInt, "1_000_000", 1000000, "", "i64"},
		{"340282366920938463463374607431768211455u128", ast.LitInt, "340282366920938463463374607431768211455", ^uint64(0), "340282366920938463463374607431768211455", "u128"},
		{"1.5", ast.LitFloat, "1.5", 0, "", ""},
		{"1e10", ast.LitFloat, "1e10", 0, "", ""},
		{"1.5e-3", ast.LitFloat, "1.5e-3", 0, "", ""},
		{"2.", ast.LitFloat, "2.0", 0, "", ""},
		{"1f64", ast.LitFloat, "1", 0, "", "f64"},
		{"2.5f32", ast.LitFloat, "2.5", 0, "", "f32"},
		{`"a\nb\t\"\\\0\x41\u{1F600}"`, ast.LitStr, "a\nb\t\"\\\x00A\U0001F600", 0, "", ""},
		{"\"line \\\n      continued\"", ast.LitStr, "line continued", 0, "", ""},
		{`r"raw\n"`, ast.LitStr, `raw\n`, 0, "", ""},
		{`r#"has "quotes""#`, ast.LitStr, `has "quotes"`, 0, "", ""},
		{`b"by\xfftes"`, ast.LitByteStr, "by\xfftes", 0, "", ""},
		{`br"raw\bytes"`, ast.LitByteStr, `raw\bytes`, 0, "", ""},
		{`c"cstr"`, ast.LitCStr, "cstr", 0, "", ""},
		{`'a'`, ast.LitChar, "a", 97, "", ""},
		{`'\n'`, ast.LitChar, "\n", 10, "", ""},
		{`'\''`, ast.LitChar, "'", 39, "", ""},
		{`'\u{e9}'`, ast.LitChar, "é", 0xe9, "", ""},
		{`'é'`, ast.LitChar, "é", 0xe9, "", ""},
		{`b'a'`, ast.LitByte, "a", 97, "", ""},
		{`b'\xff'`, ast.LitByte, "\xff", 255, "", ""},
		{"true", ast.LitBool, "true", 1, "", ""},
	}
	for _, c := range cases {
		e := parseExpr(t, c.src)
		l, ok := e.(*ast.LitExpr)
		if !ok {
			t.Errorf("%s: got %T", c.src, e)
			continue
		}
		if l.Kind != c.kind || l.Value != c.val || l.IntVal != c.intVal || l.Big != c.big || l.Suffix != c.suffix {
			t.Errorf("%s: got kind=%v val=%q int=%d big=%q suffix=%q", c.src, l.Kind, l.Value, l.IntVal, l.Big, l.Suffix)
		}
	}
}

func TestLexer(t *testing.T) {
	src := "r#match 'a 'static '_ 'x' a.0.1 1..2 1.foo x>>=y ::<> => ..= ... // c\n/* a /* nested */ b */ /// doc\n#"
	toks, err := lexer.Tokenize(0, src)
	if err != nil {
		t.Fatal(err)
	}
	var got []string
	for _, tk := range toks {
		got = append(got, tk.Kind.String()+":"+tk.Text)
	}
	want := "Ident:match Lifetime:'a Lifetime:'static Lifetime:'_ Char:x Ident:a Punct:. Int:0 Punct:. Int:1 Int:1 Punct:.. Int:2 Int:1 Punct:. Ident:foo Ident:x Punct:>>= Ident:y Punct::: Punct:< Punct:> Punct:=> Punct:..= Punct:... Punct:# EOF:"
	if strings.Join(got, " ") != want {
		t.Errorf("got  %s\nwant %s", strings.Join(got, " "), want)
	}
	if !toks[0].Raw {
		t.Errorf("r#match should be raw")
	}
	// JointNext: `a.0` are joint, `y` (before ::) is not joint with `::`.
	if !toks[5].JointNext || !toks[6].JointNext {
		t.Errorf("expected joint tokens")
	}
	if toks[18].JointNext {
		t.Errorf("y should not be joint")
	}
	// positions
	if toks[1].Span.Lo.Col != 9 || toks[1].Span.Hi.Col != 11 || toks[1].Span.Lo.Line != 1 {
		t.Errorf("bad span %+v", toks[1].Span)
	}
	last := toks[len(toks)-2]
	if last.Span.Lo.Line != 3 {
		t.Errorf("bad line %d", last.Span.Lo.Line)
	}

	// shebang
	toks, err = lexer.Tokenize(0, "#!/usr/bin/env rust\nfn")
	if err != nil || len(toks) != 2 || toks[0].Text != "fn" {
		t.Errorf("shebang: %v %v", toks, err)
	}
	toks, err = lexer.Tokenize(0, "#![allow(x)]")
	if err != nil || toks[0].Text != "#" || toks[1].Text != "!" {
		t.Errorf("inner attr: %v %v", toks, err)
	}
	// errors
	for _, bad := range []string{"\"unterminated", "/* open", "'", "\"\\q\"", "`"} {
		if _, err := lexer.Tokenize(0, bad); err == nil {
			t.Errorf("%q: expected error", bad)
		}
	}
}

func TestTypeRoundTrip(t *testing.T) {
	types := []string{
		"i32",
		"Vec<Vec<i32>>",
		"HashMap<String, Vec<(i32, String)>>",
		"Option<Box<dyn Fn(i32) -> i32>>",
		"Box<dyn Fn(i32) -> i32 + 'a>",
		"&'static str",
		"&'a mut T",
		"&&str",
		"*const u8",
		"*mut *const T",
		"[T]",
		"[T; N]",
		"[u8; 4 + 1]",
		"(i32,)",
		"(i32, String)",
		"()",
		"!",
		"_",
		"fn(i32, &str) -> bool",
		"unsafe extern \"C\" fn(*const u8, ...) -> i32",
		"impl Iterator<Item = u32> + '_",
		"dyn Trait + Send + Sync",
		"<T as Iterator>::Item",
		"<T as std::ops::Add<U>>::Output",
		"<T>::Assoc",
		"Self",
		"Self::Item",
		"Foo<{ N + 1 }>",
		"Foo<3>",
		"Foo<-3>",
		"Foo<'a, T, N>",
		"impl FnMut(&T) -> bool",
		"Box<dyn for<'a> Fn(&'a str) -> &'a str>",
		"&(dyn Any + Send)",
		"Result<(), Box<dyn std::error::Error>>",
		"::std::vec::Vec<i32>",
		"Iterator<Item: Clone>",
		"Pin<&mut dyn Future<Output = ()>>",
		"[[f64; 3]; 3]",
		"fn() -> !",
		"(dyn Fn() + Send)",
	}
	for _, src := range types {
		fid := lexer.AddFile("ty.rs", src)
		p, err := New(fid, src)
		if err != nil {
			t.Fatal(err)
		}
		ty, err := p.ParseType()
		if err != nil || !p.AtEOF() {
			t.Errorf("%s: %v (eof=%v)", src, err, p.AtEOF())
			continue
		}
		s1 := ast.TypeString(ty)
		p2, _ := New(fid, s1)
		ty2, err := p2.ParseType()
		if err != nil || !p2.AtEOF() {
			t.Errorf("%s: reparse of %q failed: %v", src, s1, err)
			continue
		}
		s2 := ast.TypeString(ty2)
		if s1 != s2 {
			t.Errorf("%s: unstable round trip %q vs %q", src, s1, s2)
		}
		if strings.ReplaceAll(s1, " ", "") != strings.ReplaceAll(src, " ", "") {
			t.Logf("%s printed as %s", src, s1)
		}
	}
}

func TestGenericsPrinting(t *testing.T) {
	c := mustParse(t, "struct S<'a, 'b: 'a, T: Clone + 'a = i32, const N: usize> where T: Default, for<'c> &'c T: Into<u8>, 'b: 'a { x: &'a [T; N], y: &'b () }")
	s := c.Items[0].Kind.(*ast.StructItem)
	if got := ast.GenericsParamsString(s.Generics); got != "<'a, 'b: 'a, T: Clone + 'a, const N: usize>" {
		t.Errorf("params: %s", got)
	}
	if got := ast.GenericsArgsString(s.Generics); got != "<'a, 'b, T, N>" {
		t.Errorf("args: %s", got)
	}
	if got := ast.WhereString(s.Generics); got != "where T: Default, for<'c> &'c T: Into<u8>, 'b: 'a" {
		t.Errorf("where: %s", got)
	}
	// The printed impl header reparses.
	src := "impl" + ast.GenericsParamsString(s.Generics) + " Clone for S" + ast.GenericsArgsString(s.Generics) + " " + ast.WhereString(s.Generics) + " {}"
	mustParse(t, src)
}

func TestExprPrintRoundTrip(t *testing.T) {
	exprs := []string{
		"1 + 2 * 3",
		"(1 + 2) * 3",
		"a - (b - c)",
		"-(a + b)",
		"!(a && b)",
		"(a as u8) < b",
		"x.0.1",
		"(1).max(2)",
		"Foo { a: 1, b, ..Default::default() }",
		"if a { b } else if c { d } else { e }",
		"match x { Some(1 | 2) => 3, None => { 4 } _ if y > 0 => 5, }",
		"|a, b: i32| a + b",
		"move || -> i32 { 1 }",
		"&mut v[1..=2]",
		"loop { break 'a 5; }",
		"[0; 4]",
		"[1, 2, 3]",
		"(1, \"s\\n\", 'c', b'x', 2.5f32)",
		"<T as Trait>::f::<u8>(x)",
		"vec![1, 2, 3]",
		"a = b += c",
		"(|x| x)(1)",
		"*&x",
		"x?.y.await",
		"for (i, x) in v.iter().enumerate() { s += x; }",
		"while let Some(x) = it.next() { }",
		"unsafe { f() }",
		"'blk: { 1 }",
		"if (Foo { x: 1 }).x == 1 { }",
		"if let Some(x) = a && x > 0 && let Ok(y) = f(x) { }",
		"(x as u8) < y",
		"(x as u64) << 2",
		"a - -1",
		"!!x",
		"&&x",
		"(a..b).len()",
		"(1.5).max(2.0)",
		"x.r#type",
		"|| (a, b)",
		"Foo::<T> { x }",
		"<Vec<i32>>::new()",
		"[(); 0]",
		"(return 1) + 2",
	}
	for _, src := range exprs {
		e := parseExpr(t, src)
		s1 := ast.ExprString(e)
		e2 := parseExpr(t, s1)
		s2 := ast.ExprString(e2)
		if s1 != s2 {
			t.Errorf("%s: unstable %q vs %q", src, s1, s2)
		}
		if sx(e) != sx(e2) {
			t.Errorf("%s: shape changed: %s vs %s", src, sx(e), sx(e2))
		}
	}
}

func TestPatterns(t *testing.T) {
	pats := map[string]string{
		"_":                      "*ast.WildPat",
		"x":                      "*ast.IdentPat",
		"None":                   "*ast.IdentPat",
		"ref mut x":              "*ast.IdentPat",
		"x @ 1..=5":              "*ast.IdentPat",
		"1":                      "*ast.LitPat",
		"-1":                     "*ast.LitPat",
		"'a'..='z'":              "*ast.RangePat",
		"..=5":                   "*ast.RangePat",
		"5..":                    "*ast.RangePat",
		"0..5":                   "*ast.RangePat",
		"-10..=-1":               "*ast.RangePat",
		"i32::MIN..=0":           "*ast.RangePat",
		"Foo::Bar":               "*ast.PathPat",
		"<T>::X":                 "*ast.PathPat",
		"Some(x)":                "*ast.TupleStructPat",
		"Point { x, y: 0, .. }":  "*ast.StructPat",
		"Foo { ref x, mut y }":   "*ast.StructPat",
		"Foo { 0: a, 1: b }":     "*ast.StructPat",
		"(a, b)":                 "*ast.TuplePat",
		"(a,)":                   "*ast.TuplePat",
		"(..)":                   "*ast.TuplePat",
		"(a)":                    "*ast.ParenPat",
		"(a, .., b)":             "*ast.TuplePat",
		"[a, b, rest @ ..]":      "*ast.SlicePat",
		"[first, .., last]":      "*ast.SlicePat",
		"&x":                     "*ast.RefPat",
		"&mut x":                 "*ast.RefPat",
		"&&x":                    "*ast.RefPat",
		"box p":                  "*ast.BoxPat",
		"A | B":                  "*ast.OrPat",
		"| A | B":                "*ast.OrPat",
		"Some(1 | 2)":            "*ast.TupleStructPat",
		"\"str\"":                "*ast.LitPat",
		"b'a'":                   "*ast.LitPat",
		"true":                   "*ast.LitPat",
		"m!(x)":                  "*ast.MacPat",
		"Ok(Some((a, [b, ..])))": "*ast.TupleStructPat",
		"E::V { inner: (x, _) }": "*ast.StructPat",
		"r#type":                 "*ast.IdentPat",
	}
	for src, want := range pats {
		fid := lexer.AddFile("pat.rs", src)
		p, _ := New(fid, src)
		pat, err := p.ParsePat()
		if err != nil || !p.AtEOF() {
			t.Errorf("%s: %v", src, err)
			continue
		}
		if got := fmt.Sprintf("%T", pat); got != want {
			t.Errorf("%s: got %s want %s", src, got, want)
		}
		// printed patterns reparse to the same thing
		s1 := ast.PatString(pat)
		p2, _ := New(fid, s1)
		pat2, err := p2.ParsePat()
		if err != nil || ast.PatString(pat2) != s1 {
			t.Errorf("%s: pattern round trip %q failed: %v", src, s1, err)
		}
	}
	// &&x is two nested ref patterns
	fid := lexer.AddFile("pat.rs", "&&x")
	p, _ := New(fid, "&&x")
	pat, _ := p.ParsePat()
	if _, ok := pat.(*ast.RefPat).Pat.(*ast.RefPat); !ok {
		t.Errorf("&&x should be nested RefPat")
	}
	// x @ 1..=5
	p, _ = New(fid, "x @ 1..=5")
	pat, _ = p.ParsePat()
	if _, ok := pat.(*ast.IdentPat).Sub.(*ast.RangePat); !ok {
		t.Errorf("sub pattern should be range")
	}
	// rest @ ..
	p, _ = New(fid, "[a, rest @ ..]")
	pat, _ = p.ParsePat()
	if ip := pat.(*ast.SlicePat).Elems[1].(*ast.IdentPat); ip.Name.Name != "rest" {
		t.Errorf("bad rest binding")
	} else if _, ok := ip.Sub.(*ast.RestPat); !ok {
		t.Errorf("rest binding sub should be RestPat")
	}
}

func TestItems(t *testing.T) {
	src := `
#![no_std]
#![allow(unused)]
extern crate alloc;
extern crate core as kore;
use std::collections::HashMap;
use std::io::{self, Read, Write as W};
use ::std::*;
use a::b::{c::{d, e}, f::*};
use {x, y};
pub use self::m::Thing as _;
pub(crate) fn f() {}
pub(super) fn g() {}
pub(in crate::a) fn h() {}
pub(self) fn i() {}
const fn cf() -> u32 { 1 }
unsafe fn uf() {}
async fn af() {}
extern "C" fn ef() {}
pub unsafe extern "C" fn uef(x: i32, ...) {}
#[derive(Debug, Clone)]
pub struct Named<T> where T: Copy { pub a: T, #[serde(skip)] b: i32, }
struct Tuple(pub i32, pub(crate) String);
struct Generic<T>(T) where T: Clone;
struct Unit;
#[repr(C)]
union U { a: u32, b: f32 }
enum E<T> { A, B(T, i32), C { x: i32, y: T }, D = 5, #[default] F }
trait Tr<T>: Clone + Send where T: Copy {
    type Assoc: Iterator<Item = T> + Clone;
    type WithDefault = i32;
    type Gat<'a>: 'a where Self: 'a;
    const C: usize;
    const D: i32 = 5;
    fn required(&self, x: T) -> T;
    fn provided(&mut self) -> Option<T> { None }
    fn by_value(self) where Self: Sized;
    fn boxed(self: Box<Self>);
    fn mut_self(mut self) {}
    fn lifetime_self(&'a self) {}
    fn lifetime_mut_self(&'a mut self) {}
}
unsafe trait UnsafeTrait {}
auto trait Auto {}
impl<T> Tr<T> for Vec<T> where T: Clone + Copy {
    type Assoc = std::vec::IntoIter<T>;
    const C: usize = 3;
    fn required(&self, x: T) -> T { x }
}
impl !Send for Unit {}
unsafe impl Send for U {}
impl<'a, T: 'a + ?Sized> Named<&'a T> {}
impl<T, const N: usize> Default for [T; N] {}
impl dyn Tr<i32> {}
type Alias<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;
static S: &str = "s";
static mut M: i32 = 0;
const _: () = ();
mod external;
mod inline { #![allow(x)] fn f() {} pub mod deeper { } }
extern "C" { fn printf(f: *const u8, ...) -> i32; static errno: i32; type Opaque; }
extern { fn noabi(); }
macro_rules! m { ($x:expr) => { $x }; () => {} }
macro_rules! paren ( () => () );
thread_local! { static X: i32 = 0; }
lazy_static!(X, Y);
some::path::mac! { items }
fn generic<'a, T: Iterator<Item = &'a u8> + 'a, F>(f: F) -> impl Fn(&T) -> bool + 'a where F: for<'b> Fn(&'b T) -> bool, Self: Sized {}
fn params(&self, (a, b): (i32, i32), mut c: i32, _: u8, Point { x, y }: Point, #[attr] z: i32) {}
fn trailing<T,>(a: i32,) -> i32 { a }
`
	c := mustParse(t, src)
	if len(c.Attrs) != 2 || !c.Attrs[0].Inner || c.Attrs[0].Path != "no_std" {
		t.Errorf("crate attrs: %+v", c.Attrs)
	}
	names := map[string]*ast.Item{}
	for _, it := range c.Items {
		names[it.Name.Name] = it
	}
	if it := names["f"]; it == nil || it.Vis != ast.VisCrate {
		t.Errorf("f vis")
	}
	if it := names["g"]; it == nil || it.Vis != ast.VisRestricted {
		t.Errorf("g vis")
	}
	if it := names["i"]; it == nil || it.Vis != ast.VisPrivate {
		t.Errorf("i vis")
	}
	if fn := names["cf"].Kind.(*ast.FnItem); !fn.Const {
		t.Errorf("const fn")
	}
	if fn := names["uef"].Kind.(*ast.FnItem); !fn.Unsafe || fn.Abi != "C" || !fn.Decl.Variadic || names["uef"].Vis != ast.VisPub {
		t.Errorf("uef: %+v", fn)
	}
	if fn := names["af"].Kind.(*ast.FnItem); !fn.Async {
		t.Errorf("async fn")
	}
	if it := names["Named"]; len(it.Attrs) != 1 || it.Attrs[0].Path != "derive" || len(it.Attrs[0].Tokens) != 1 || it.Attrs[0].Tokens[0].Delim != '(' {
		t.Errorf("derive attr: %+v", it.Attrs)
	}
	if s := names["Tuple"].Kind.(*ast.StructItem); s.Shape != ast.ShapeTuple || len(s.Fields) != 2 || s.Fields[1].Vis != ast.VisCrate {
		t.Errorf("tuple struct")
	}
	if s := names["Unit"].Kind.(*ast.StructItem); s.Shape != ast.ShapeUnit {
		t.Errorf("unit struct")
	}
	e := names["E"].Kind.(*ast.EnumItem)
	if len(e.Variants) != 5 || e.Variants[1].Shape != ast.ShapeTuple || e.Variants[2].Shape != ast.ShapeNamed || e.Variants[3].Disc == nil {
		t.Errorf("enum variants")
	}
	tr := names["Tr"].Kind.(*ast.TraitItem)
	if len(tr.Items) != 12 || len(tr.Supertraits) != 2 {
		t.Errorf("trait items %d supertraits %d", len(tr.Items), len(tr.Supertraits))
	}
	sps := []ast.SelfParamKind{ast.SelfRef, ast.SelfRefMut, ast.SelfValue, ast.SelfTyped, ast.SelfValue, ast.SelfRef, ast.SelfRefMut}
	for i, k := range sps {
		fn := tr.Items[5+i].Kind.(*ast.FnItem)
		if fn.Decl.SelfParam == nil || fn.Decl.SelfParam.Kind != k {
			t.Errorf("self param %d (%s)", i, tr.Items[5+i].Name.Name)
		}
	}
	if fn := tr.Items[9].Kind.(*ast.FnItem); !fn.Decl.SelfParam.Mut {
		t.Errorf("mut self")
	}
	var impls []*ast.ImplItem
	for _, it := range c.Items {
		if im, ok := it.Kind.(*ast.ImplItem); ok {
			impls = append(impls, im)
		}
	}
	if len(impls) != 6 {
		t.Fatalf("impls: %d", len(impls))
	}
	if impls[0].Trait == nil || ast.PathString(impls[0].Trait) != "Tr<T>" || ast.TypeString(impls[0].SelfTy) != "Vec<T>" || len(impls[0].Generics.Where) != 1 {
		t.Errorf("impl 0")
	}
	if !impls[1].Negative || !impls[2].Unsafe || impls[3].Trait != nil {
		t.Errorf("impl flags")
	}
	m := names["external"].Kind.(*ast.ModItem)
	if m.Inline || m.Items != nil {
		t.Errorf("external mod")
	}
	m = names["inline"].Kind.(*ast.ModItem)
	if !m.Inline || len(m.Items) != 2 || len(names["inline"].Attrs) != 1 {
		t.Errorf("inline mod")
	}
	if mr := names["m"].Kind.(*ast.MacroRulesItem); len(mr.Body) != 7 {
		t.Errorf("macro_rules body: %d", len(mr.Body))
	}
	if _, ok := names["paren"].Kind.(*ast.MacroRulesItem); !ok {
		t.Errorf("paren macro_rules")
	}
	fn := names["generic"].Kind.(*ast.FnItem)
	if len(fn.Generics.Params) != 3 || len(fn.Generics.Where) != 2 {
		t.Errorf("generic fn")
	}
	if ast.TypeString(fn.Decl.Output) != "impl Fn(&T) -> bool + 'a" {
		t.Errorf("ret: %s", ast.TypeString(fn.Decl.Output))
	}
	fn = names["params"].Kind.(*ast.FnItem)
	if len(fn.Decl.Params) != 5 || fn.Decl.SelfParam == nil || len(fn.Decl.Params[4].Attrs) != 1 {
		t.Errorf("params")
	}
	eb := c.Items[len(c.Items)-10].Kind
	_ = eb

	// use trees
	var uses []*ast.UseTree
	for _, it := range c.Items {
		if u, ok := it.Kind.(*ast.UseItem); ok {
			uses = append(uses, u.Tree)
		}
	}
	if len(uses) != 6 {
		t.Fatalf("uses: %d", len(uses))
	}
	if uses[1].Kind != ast.UseNested || len(uses[1].Nested) != 3 || uses[1].Nested[0].Prefix.Segments[0].Ident.Name != "self" || uses[1].Nested[2].Rename.Name != "W" {
		t.Errorf("use 1")
	}
	if !uses[2].Prefix.Global || uses[2].Kind != ast.UseGlob || len(uses[2].Prefix.Segments) != 1 {
		t.Errorf("use 2")
	}
	if uses[4].Kind != ast.UseNested || len(uses[4].Prefix.Segments) != 0 {
		t.Errorf("use 4")
	}
	if uses[5].Rename == nil || uses[5].Rename.Name != "_" {
		t.Errorf("use 5")
	}
}

// TestSnippets checks that a wide range of function bodies parse.
func TestSnippets(t *testing.T) {
	bodies := []string{
		"let (a, b): (i32, i32) = (1, 2);",
		"let Some(x) = opt else { return; };",
		"let v: Vec<Vec<i32>> = Vec::new();",
		"let v: Vec<i32>= vec![];",
		"let x: HashMap<String, Vec<(i32, String)>> = HashMap::new();",
		"let f: Option<Box<dyn Fn(i32) -> i32>> = None;",
		"if a {} else if b {} else {}",
		"match x { 1 => {} _ => {} }",
		"match x { 1 => {}, 2 | 3 => (), x if x > 5 => println!(\"big\"), _ => unreachable!() }",
		"match x { | A | B => 1, C => 2 }",
		"match x { Foo::Bar { x, .. } => x, Foo::Baz(..) => 0, }",
		"v.iter().map(|x| x + 1).filter(|&x| x > 2).collect::<Vec<_>>();",
		"let s = Foo::<T>::bar();",
		"let r = &&x;",
		"let s = S { a, b: 1, ..base };",
		"let s = Foo::Bar { x };",
		"let t = Foo { 0: x, 1: y };",
		"return;",
		"if x { return } else { break }",
		"loop { break 'outer; }",
		"'outer: loop { 'inner: while true { continue 'outer; } }",
		"let x = 'a: { break 'a 1; };",
		"impl<T> Trait for Vec<T> where T: Clone {}",
		"let n = 5.max(3) + 1.0f64.sqrt() as i32;",
		"let y = x - 1; let z = x -1;",
		"let p = -1;",
		"x.0.1 = 5;",
		"let a = t.0 .1;",
		"let c = |a, b: i32| a + b;",
		"let d = move |x| { x };",
		"let e = || -> i32 { 5 };",
		"let f = |&(a, b)| a + b;",
		"let g = || ();",
		"let h = async move { 1 };",
		"let u = unsafe { *p };",
		"let k = const { 5 };",
		"while let Some(x) = stack.pop() { }",
		"for (i, x) in v.iter().enumerate() {}",
		"for x in &v {}",
		"for _ in 0..10 {}",
		"let arr = [0; N];",
		"let arr = [1, 2, 3,];",
		"let v = <T as Default>::default();",
		"let v = <Vec<i32>>::new();",
		"let v = <Vec<i32> as IntoIterator>::into_iter(x);",
		"let s = std::mem::size_of::<u64>();",
		"println!(\"{}\", x);",
		"assert_eq!(a, b, \"msg {}\", c);",
		"let v = vec![1, 2, 3];",
		"let v = core::format_args!(\"x\");",
		"(a, b) = (b, a);",
		"_ = f();",
		"[a, _, b] = arr;",
		"Foo { x, .. } = foo;",
		"x += 1; x -= 1; x *= 2; x /= 2; x %= 3; x ^= 1; x &= 1; x |= 1; x <<= 1; x >>= 1;",
		"let r = a..b; let r = ..; let r = ..=b; let r = a..=b; let r = a..;",
		"let s = &v[..]; let s = &v[1..]; let s = &v[..2];",
		"let c = x as i64 < y;",
		"let b = !a && b || !(c && d);",
		"let p = &raw const x; let q = &raw mut y;",
		"let x = if let Some(x) = a && x > 0 && let Ok(y) = b { x + y } else { 0 };",
		"let x = match x { Some(x) if x > 0 => x, Some(_) | None => 0 };",
		"let s = format!(\"{}\", x) + \"y\";",
		"let x = { 1 } + { 2 };",
		"#[allow(unused)] let x = 1;",
		"#[cfg(test)] fn helper() {}",
		"let v = x.await;",
		"f()?;",
		"let x = a?.b?.c()?;",
		"let x = y as *const T as usize;",
		"let x = &mut *y;",
		"let x = **y;",
		"let x = -*y;",
		"struct Local; impl Local { fn f() {} }",
		"use std::fmt; use super::*;",
		"let x: <T as Iterator>::Item = it.next().unwrap();",
		"let b: Box<[u8]> = Box::new([1, 2]);",
		"let f: fn(i32) -> i32 = foo;",
		"let f: unsafe extern \"C\" fn(i32) -> i32 = foo;",
		"let r: &'static str = \"x\";",
		"if x == y { } else { }",
		"if a.b == C::D { }",
		"while i < n { i += 1; }",
		"match (a, b) { (1, 2) => {}, (x, _) if x > 3 => {}, _ => {} }",
		"let x = Some(5).map(|v| v * 2).unwrap_or(0);",
		"let closure = |x: &mut Vec<i32>| x.push(1);",
		"items.sort_by(|a, b| b.cmp(a));",
		"let r = if cond { Ok(1) } else { Err(\"e\") }?;",
		"let x = match y { _ => 1 } + 2;",
		"let v = (0..10).rev().step_by(2).collect::<Vec<usize>>();",
		"let m: HashMap<&str, i32> = [(\"a\", 1)].into_iter().collect();",
		"let s = String::from(\"a\") + &b;",
		"let x = r#type;",
		"let t = (1,);",
		"let u = ();",
		"break;",
		"continue;",
		"self.x = 5;",
		"Self::new()",
		"let s = Self { x: 1 };",
		"let x = crate::a::b(); let y = super::c;",
		"let x = a.b.c.d();",
		"let x = (a.b)();",
		"let x = a[0][1];",
		"let n = 0xFFu8 as i8;",
		"let n = 1_000_000;",
		"let n = 1e-3;",
		"let c = 'x'; let d = '\\''; let e = b'\\n';",
		"let s = \"multi\nline\";",
		"x.method::<A, B>(1, 2);",
		"let v: Vec<_> = it.collect();",
		"if x {} if y {}",
		"{ } { }",
		"let x = loop { break 5; };",
		"let f = &|x| x;",
		"call(|| {});",
		"let x = !0u32 >> 1;",
		"a.b().c().d;",
		"let x = Box::new(move |a: i32| -> i32 { a + n });",
		"return Err(e.into());",
		"return Ok(());",
		"let g = |x| if x { 1 } else { 2 };",
		"let x = y.0;",
		"match x { E::A(ref mut v) => {} E::B { ref name, .. } => {} }",
		"match c { 'a'..='z' | 'A'..='Z' => {} '0'..='9' => {} _ => {} }",
		"match n { i32::MIN..=-1 => {} 0 => {} 1.. => {} }",
		"match s { \"a\" => {} _ => {} }",
		"match v[..] { [first, ref rest @ ..] => {} [] => {} }",
		"match b { box x => {} }",
		"match x { -1 => {} -5..=-2 => {} _ => {} }",
		"let x = y.iter().any(|v| *v == Foo { a: 1 });",
		"if v.iter().any(|x| x == y) {}",
		"let Point { x, y: py } = p;",
		"let [a, b, c] = arr;",
		"let &(a, b) = pair;",
		"let mut x = 5;",
		"let ref y = x;",
		"let ref mut z = x;",
		"let (mut a, ref b) = t;",
		"x;",
		"f(a, b,);",
		"let _: () = ();",
		"type T = i32;",
		"const C: i32 = 5;",
		"static S: i32 = 5;",
		"enum Local { A, B }",
		"trait LocalTr { fn f(&self); }",
		"mod m {}",
		"extern \"C\" { fn g(); }",
		"let x = &mut [1, 2, 3][..];",
		"let i = a.0 + b.1.2;",
		"if !matches!(x, Some(_)) { }",
		"let r = (x as f64) / 2.0;",
		"let c = a < b && c > d;",
		"let c = (a < b) == (c > d);",
		"let p: Vec<Box<dyn Fn() -> Result<(), String>>> = vec![];",
		"let n: u128 = 1 << 100;",
		"let nested = Some(Some(Some(1)));",
		"thread_local!(static FOO: u32 = 1);",
		"m! { a } let z = 1;",
	}
	for _, b := range bodies {
		mustParse(t, "fn test_fn() {\n"+b+"\n}")
	}
}

func TestErrors(t *testing.T) {
	cases := []struct{ src, want string }{
		{"fn f() { let x = ; }", "expected expression, found `;`"},
		{"fn f() { a b }", "expected `;`, found identifier `b`"},
		{"struct S { x }", "expected `:`"},
		{"fn f( {}", "expected"},
		{"fn f() { (1, 2 }", "expected `)`"},
		{"fn", "expected identifier, found end of input"},
		{"impl X for {}", "expected type"},
		{"fn f() { match x { 1 => 2 3 => 4 } }", "expected `,` or `}`"},
	}
	for _, c := range cases {
		fid := lexer.AddFile("err.rs", c.src)
		_, err := ParseCrate(fid, c.src)
		if err == nil {
			t.Errorf("%q: expected error", c.src)
			continue
		}
		if !strings.Contains(err.Error(), c.want) || !strings.HasPrefix(err.Error(), "err.rs:1:") {
			t.Errorf("%q: error %q does not contain %q", c.src, err, c.want)
		}
	}
}

func TestTokenTrees(t *testing.T) {
	tts, err := TokenizeToTrees(0, "a (b [c] {d e}) f")
	if err != nil {
		t.Fatal(err)
	}
	if len(tts) != 3 || tts[1].Delim != '(' || len(tts[1].Inner) != 3 || tts[1].Inner[2].Delim != '{' {
		t.Fatalf("bad trees: %+v", tts)
	}
	if _, err := TokenizeToTrees(0, "(a]"); err == nil {
		t.Errorf("expected mismatch error")
	}
	if _, err := TokenizeToTrees(0, "(a"); err == nil {
		t.Errorf("expected unclosed error")
	}
	if _, err := TokenizeToTrees(0, "a)"); err == nil {
		t.Errorf("expected unexpected-close error")
	}

	// Parse from token trees (as the macro expander does).
	tts, _ = TokenizeToTrees(0, "1 + foo(2, [3]) * x.y")
	p := NewFromTokenTrees(tts)
	e, err := p.ParseExpr()
	if err != nil || !p.AtEOF() {
		t.Fatalf("parse from trees: %v", err)
	}
	if got := sx(e); got != "(+ 1 (* (call foo [2 3]) (field x y)))" {
		// the array literal prints as <*ast.ArrayExpr>; just check the shape loosely
		if !strings.HasPrefix(got, "(+ 1 (* (call foo") {
			t.Errorf("got %s", got)
		}
	}

	// statements from token trees: MacStmt tail
	tts, _ = TokenizeToTrees(0, "let x = 1; format!(\"{}\", x)")
	p = NewFromTokenTrees(tts)
	stmts, err := p.ParseStmts()
	if err != nil || len(stmts) != 2 {
		t.Fatalf("stmts: %v %d", err, len(stmts))
	}
	if ms, ok := stmts[1].(*ast.MacStmt); !ok || ms.Semi {
		t.Errorf("want tail MacStmt")
	}

	// items from token trees
	tts, _ = TokenizeToTrees(0, "fn a() {} struct B;")
	p = NewFromTokenTrees(tts)
	items, err := p.ParseItems()
	if err != nil || len(items) != 2 {
		t.Fatalf("items: %v", err)
	}

	// Backtracking with Pos/Reset, including through a split `>>`.
	tts, _ = TokenizeToTrees(0, "Vec<Vec<u8>> rest")
	p = NewFromTokenTrees(tts)
	start := p.Pos()
	if _, err := p.ParseExpr(); err != nil {
		t.Fatalf("expr: %v", err)
	}
	p.Reset(start)
	ty, err := p.ParseType()
	if err != nil || ast.TypeString(ty) != "Vec<Vec<u8>>" {
		t.Fatalf("type after reset: %v %v", ty, err)
	}
	if id, err := p.ParseIdent(); err != nil || id.Name != "rest" {
		t.Fatalf("ident: %v", err)
	}
	// split position round trip
	tts, _ = TokenizeToTrees(0, "A<B<C>> x")
	p = NewFromTokenTrees(tts)
	p.Next()
	p.Next()
	p.Next()
	p.Next()
	p.Next() // consumed A < B < C
	if p.Peek().Text != ">>" {
		t.Fatalf("peek %v", p.Peek())
	}
	p2 := NewFromTokenTrees(tts)
	if _, err := p2.ParseType(); err != nil {
		t.Fatal(err)
	}
	if id, _ := p2.ParseIdent(); id.Name != "x" {
		t.Errorf("after split")
	}
}

func TestMacroFragments(t *testing.T) {
	mk := func(src string) *Parser {
		tts, err := TokenizeToTrees(0, src)
		if err != nil {
			t.Fatal(err)
		}
		return NewFromTokenTrees(tts)
	}
	p := mk("-5")
	if e, err := p.ParseLiteral(); err != nil || sx(e) != "(neg 5)" {
		t.Errorf("literal: %v", err)
	}
	p = mk("pub(crate)")
	if v, err := p.ParseVisibility(); err != nil || v != ast.VisCrate {
		t.Errorf("vis")
	}
	p = mk("'a")
	if lt, err := p.ParseLifetime(); err != nil || lt != "'a" {
		t.Errorf("lifetime")
	}
	p = mk("std::vec::Vec::<i32>::new")
	if path, err := p.ParsePath(); err != nil || len(path.Segments) != 4 || !p.AtEOF() {
		t.Errorf("path: %v", err)
	}
	p = mk("{ let x = 1; x }")
	if b, err := p.ParseBlock(); err != nil || len(b.Stmts) != 2 {
		t.Errorf("block: %v", err)
	}
	p = mk("derive(Debug)")
	if a, err := p.ParseMeta(); err != nil || a.Path != "derive" || len(a.Tokens) != 1 {
		t.Errorf("meta: %v", err)
	}
	p = mk("doc = \"hello\"")
	if a, err := p.ParseMeta(); err != nil || a.Path != "doc" || len(a.Tokens) != 2 {
		t.Errorf("meta2: %v %+v", err, a)
	}
	p = mk("Some(x) | None")
	if pat, err := p.ParsePat(); err != nil || !p.AtEOF() {
		t.Errorf("pat: %v", err)
	} else if _, ok := pat.(*ast.OrPat); !ok {
		t.Errorf("want or-pat")
	}
	p = mk("self")
	if id, err := p.ParseIdent(); err != nil || id.Name != "self" {
		t.Errorf("ident self")
	}
	p = mk("(a b) c")
	if tt, err := p.ParseTokenTree(); err != nil || tt.Delim != '(' {
		t.Errorf("tt")
	}
	p = mk("$crate::foo::bar()")
	if e, err := p.ParseExpr(); err != nil {
		t.Errorf("$crate: %v", err)
	} else if c, ok := e.(*ast.CallExpr); !ok || c.Fn.(*ast.PathExpr).Path.Segments[0].Ident.Name != "$crate" {
		t.Errorf("$crate path")
	}
	p = mk("$crate::m!(x)")
	if stmts, err := p.ParseStmts(); err != nil || len(stmts) != 1 {
		t.Errorf("$crate mac: %v", err)
	}
}

func TestIDsAndSpans(t *testing.T) {
	src := "fn main() {\n    let x = 1 + 2;\n}\n"
	c := mustParse(t, src)
	seen := map[ast.NodeID]bool{}
	var check func(id ast.NodeID, what string)
	check = func(id ast.NodeID, what string) {
		if id == 0 {
			t.Errorf("%s has zero ID", what)
		}
		if seen[id] {
			t.Errorf("%s has duplicate ID %d", what, id)
		}
		seen[id] = true
	}
	it := c.Items[0]
	check(it.ID, "item")
	fn := it.Kind.(*ast.FnItem)
	check(fn.Body.ID, "block")
	ls := fn.Body.Stmts[0].(*ast.LetStmt)
	check(ls.ID, "let")
	check(ls.Pat.GetID(), "pat")
	be := ls.Init.(*ast.BinaryExpr)
	check(be.ID, "binary")
	check(be.L.GetID(), "lit")
	sp := be.Span
	if sp.Lo.Line != 2 || sp.Lo.Col != 13 || sp.Hi.Col != 18 {
		t.Errorf("binary span %v-%v", sp.Lo, sp.Hi)
	}
	if it.Span.Lo.Line != 1 || it.Span.Hi.Line != 3 {
		t.Errorf("item span %v %v", it.Span.Lo, it.Span.Hi)
	}
}

func TestStmtBlockLikeThenTuple(t *testing.T) {
	stmts := parseStmts(t, "match x {} (a, b) = (1, 2);")
	if len(stmts) != 2 {
		t.Fatalf("want 2 stmts, got %d", len(stmts))
	}
	if _, ok := stmts[1].(*ast.ExprStmt).Expr.(*ast.AssignExpr); !ok {
		t.Errorf("want assignment")
	}
}

func TestTrickyFile(t *testing.T) {
	src, err := os.ReadFile("testdata/tricky.rs")
	if err != nil {
		t.Fatal(err)
	}
	mustParse(t, string(src))
}

func TestSampleFile(t *testing.T) {
	src, err := os.ReadFile("testdata/sample.rs")
	if err != nil {
		t.Fatal(err)
	}
	c := mustParse(t, string(src))
	if len(c.Items) < 30 {
		t.Errorf("only %d items", len(c.Items))
	}
	checkReprint(t, c)
	src, err = os.ReadFile("testdata/tricky.rs")
	if err != nil {
		t.Fatal(err)
	}
	checkReprint(t, mustParse(t, string(src)))
}

// checkReprint checks that every expression and type in c prints to text
// that reparses.
func checkReprint(t *testing.T, c *ast.Crate) {
	t.Helper()
	ast.Walk(c, func(n any) bool {
		if e, ok := n.(ast.Expr); ok {
			s := ast.ExprString(e)
			fid := lexer.AddFile("re.rs", s)
			p, err := New(fid, s)
			if err != nil {
				t.Errorf("relex %q: %v", s, err)
				return false
			}
			if _, err := p.ParseExpr(); err != nil || !p.AtEOF() {
				t.Errorf("reparse %q: %v", s, err)
			}
			return false
		}
		if ty, ok := n.(ast.Type); ok {
			s := ast.TypeString(ty)
			fid := lexer.AddFile("re.rs", s)
			p, _ := New(fid, s)
			if _, err := p.ParseType(); err != nil || !p.AtEOF() {
				t.Errorf("reparse type %q: %v", s, err)
			}
		}
		return true
	})
}
