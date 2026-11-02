package lexer

import (
	"strings"
	"testing"
)

func kinds(t *testing.T, src string) string {
	t.Helper()
	toks, err := Tokenize(0, src)
	if err != nil {
		t.Fatalf("%q: %v", src, err)
	}
	var parts []string
	for _, tk := range toks {
		if tk.Kind == EOF {
			break
		}
		s := tk.Kind.String() + ":" + tk.Text
		if tk.Suffix != "" {
			s += "/" + tk.Suffix
		}
		parts = append(parts, s)
	}
	return strings.Join(parts, " ")
}

func TestTokenize(t *testing.T) {
	cases := []struct{ src, want string }{
		{"foo _bar r#fn ünïcode", "Ident:foo Ident:_bar Ident:fn Ident:ünïcode"},
		{"'a 'static '_ 'a' '\\n' b'x'", "Lifetime:'a Lifetime:'static Lifetime:'_ Char:a Char:\n Byte:x"},
		{"1 0x1F 0o17 0b1_0 1_000u64 7usize", "Int:1 Int:0x1F Int:0o17 Int:0b1_0 Int:1_000/u64 Int:7/usize"},
		{"1.5 1e10 1.5e-3 2. 1f64 3.0f32 1E+2", "Float:1.5 Float:1e10 Float:1.5e-3 Float:2. Float:1/f64 Float:3.0/f32 Float:1E+2"},
		{"1..2 1.foo 1..=3", "Int:1 Punct:.. Int:2 Int:1 Punct:. Ident:foo Int:1 Punct:..= Int:3"},
		{"t.0.1", "Ident:t Punct:. Int:0 Punct:. Int:1"},
		{`"a\tb" r"x\n" r#"q"q"# b"\xff" br"z" c"c"`, "Str:a\tb Str:x\\n Str:q\"q ByteStr:\xff ByteStr:z CStr:c"},
		{":: -> => == != <= >= && || += -= *= /= %= ^= &= |= <<= >>= << >> .. ... ..=",
			"Punct::: Punct:-> Punct:=> Punct:== Punct:!= Punct:<= Punct:>= Punct:&& Punct:|| Punct:+= Punct:-= Punct:*= Punct:/= Punct:%= Punct:^= Punct:&= Punct:|= Punct:<<= Punct:>>= Punct:<< Punct:>> Punct:.. Punct:... Punct:..="},
		{"# $ ? @ ~ + - * / % ^ ! & | = < > . , ; : ( ) [ ] { }",
			"Punct:# Punct:$ Punct:? Punct:@ Punct:~ Punct:+ Punct:- Punct:* Punct:/ Punct:% Punct:^ Punct:! Punct:& Punct:| Punct:= Punct:< Punct:> Punct:. Punct:, Punct:; Punct:: Punct:( Punct:) Punct:[ Punct:] Punct:{ Punct:}"},
		{"a // line\nb /* x /* y */ z */ c /// doc\n//! inner\nd /** blk */ e", "Ident:a Ident:b Ident:c Ident:d Ident:e"},
		{"\"x\\\n    y\"", "Str:xy"},
		{"\"\\u{1F600}\\0\\'\\\"\"", "Str:\U0001F600\x00'\""},
		{"#!/bin/sh\nx", "Ident:x"},
		{"#![attr]", "Punct:# Punct:! Punct:[ Ident:attr Punct:]"},
	}
	for _, c := range cases {
		if got := kinds(t, c.src); got != c.want {
			t.Errorf("%q:\n got  %s\n want %s", c.src, got, c.want)
		}
	}
}

func TestRawIdentAndJoint(t *testing.T) {
	toks, err := Tokenize(0, "r#type a.b c :: d")
	if err != nil {
		t.Fatal(err)
	}
	if !toks[0].Raw || toks[0].Text != "type" {
		t.Errorf("raw ident: %+v", toks[0])
	}
	if !toks[1].JointNext || !toks[2].JointNext || toks[3].JointNext || toks[4].JointNext {
		t.Errorf("joint flags: %v %v %v %v", toks[1].JointNext, toks[2].JointNext, toks[3].JointNext, toks[4].JointNext)
	}
	if toks[len(toks)-2].JointNext {
		t.Errorf("last token must not be joint with EOF")
	}
}

func TestPositions(t *testing.T) {
	toks, err := Tokenize(3, "fn main() {\n  let é = 1;\n}")
	if err != nil {
		t.Fatal(err)
	}
	// `é` is on line 2, col 7 (1-based, in characters)
	e := toks[6]
	if e.Text != "é" || e.Span.Lo.Line != 2 || e.Span.Lo.Col != 7 || e.Span.Hi.Col != 8 || e.Span.Lo.File != 3 {
		t.Errorf("bad position %+v", e.Span)
	}
	if e.Span.Hi.Off-e.Span.Lo.Off != 2 {
		t.Errorf("byte offsets: %+v", e.Span)
	}
}

func TestLexErrors(t *testing.T) {
	for _, bad := range []string{`"open`, "/* open", "'", `"\q"`, "`", `'\x80'`, `b"é"`, "r#\"x\"", "'\\u{110000}'"} {
		if _, err := Tokenize(0, bad); err == nil {
			t.Errorf("%q: expected error", bad)
		}
	}
}
