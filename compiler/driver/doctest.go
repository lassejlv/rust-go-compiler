package driver

// Doc-tests (like rustdoc --test): Rust code blocks in the doc comments of
// a library are compiled against it and run. All doc-tests of a crate are
// merged into one program (each in its own module, its code kept at its
// source line so panics point into the documented file); the program runs
// the test named by its first argument. If the merged program does not
// compile, every test is compiled on its own to find the failing ones.

import (
	"fmt"
	"regexp"
	"sort"
	"strings"

	"rustgo/compiler/ast"
	"rustgo/compiler/expand"
	"rustgo/compiler/lexer"
	"rustgo/compiler/parser"
)

// DocTest is one doc-test.
type DocTest struct {
	Name        string // "src/lib.rs - path::item (line N)"
	File        string // display name of the documented file
	Line        int    // line of the opening fence
	Code        string
	ShouldPanic bool
	NoRun       bool
	Ignore      bool
	CompileFail bool
	// results
	Built    bool   // compiled (as part of the merged program or alone)
	BuildErr string // compile error (when it did not compile)
	Index    int    // argument selecting the test in its program
	Exe      string
}

var fnMainRe = regexp.MustCompile(`(^|[^\w])fn\s+main\s*\(`)

// parseDocTests extracts the doc-tests from the docs of a crate.
func parseDocTests(blocks []expand.DocBlock) []*DocTest {
	var out []*DocTest
	for _, b := range blocks {
		// unindent: drop the common leading whitespace
		minInd := -1
		for _, l := range b.Lines {
			t := strings.TrimLeft(l.Text, " \t")
			if t == "" {
				continue
			}
			if ind := len(l.Text) - len(t); minInd < 0 || ind < minInd {
				minInd = ind
			}
		}
		if minInd < 0 {
			continue
		}
		lines := make([]string, len(b.Lines))
		for i, l := range b.Lines {
			if len(l.Text) >= minInd {
				lines[i] = l.Text[minInd:]
			} else {
				lines[i] = strings.TrimLeft(l.Text, " \t")
			}
		}
		for i := 0; i < len(lines); i++ {
			t := strings.TrimLeft(lines[i], " ")
			ind := len(lines[i]) - len(t)
			if ind > 3 || !(strings.HasPrefix(t, "```") || strings.HasPrefix(t, "~~~")) {
				continue
			}
			ch := t[0]
			n := 0
			for n < len(t) && t[n] == ch {
				n++
			}
			lang := strings.TrimSpace(t[n:])
			if ch == '`' && strings.Contains(lang, "`") {
				continue
			}
			var code []string
			j := i + 1
			for ; j < len(lines); j++ {
				u := strings.TrimLeft(lines[j], " ")
				m := 0
				for m < len(u) && u[m] == ch {
					m++
				}
				if m >= n && strings.TrimSpace(u[m:]) == "" && len(lines[j])-len(u) <= 3 {
					break
				}
				l := lines[j]
				for k := 0; k < ind && strings.HasPrefix(l, " "); k++ {
					l = l[1:]
				}
				code = append(code, l)
			}
			fence := b.Lines[i]
			dt := &DocTest{File: lexer.FileName(fence.File), Line: fence.Line, Code: strings.Join(code, "\n")}
			if parseLang(lang, dt) {
				name := b.Path
				if name != "" {
					name += " "
				}
				dt.Name = fmt.Sprintf("%s - %s(line %d)", dt.File, name, dt.Line)
				out = append(out, dt)
			}
			i = j
		}
	}
	sort.SliceStable(out, func(i, j int) bool { return out[i].Name < out[j].Name })
	return out
}

// parseLang reads a code block's info string; false if it is not Rust.
func parseLang(lang string, dt *DocTest) bool {
	lang = strings.TrimPrefix(strings.TrimSuffix(lang, "}"), "{")
	rust, other := false, false
	for _, tok := range strings.FieldsFunc(lang, func(r rune) bool { return r == ',' || r == ' ' || r == '\t' }) {
		switch {
		case tok == "rust":
			rust = true
		case tok == "should_panic":
			dt.ShouldPanic, rust = true, true
		case tok == "no_run":
			dt.NoRun, rust = true, true
		case tok == "ignore" || strings.HasPrefix(tok, "ignore-"):
			dt.Ignore, rust = true, true
		case tok == "compile_fail":
			dt.CompileFail, rust = true, true
		case tok == "test_harness" || tok == "standalone_crate" || tok == "allow_fail" || strings.HasPrefix(tok, "edition"):
			rust = true
		case len(tok) == 5 && tok[0] == 'E' && strings.Trim(tok[1:], "0123456789") == "":
			rust = true
		case strings.HasPrefix(tok, "."):
			// {.class}
		default:
			other = true
		}
	}
	return rust || !other
}

// docTestCode turns a doc-test's code into module items: hidden lines
// (`# `) are shown to the compiler, crate attributes and `extern crate`s
// are hoisted (returned separately, their lines blanked), and code without
// a `fn main` is wrapped in one. The result keeps the code's line count.
func docTestCode(code string) (body string, hoisted []string, hasMain bool) {
	lines := strings.Split(code, "\n")
	for i, l := range lines {
		t := strings.TrimLeft(l, " \t")
		switch {
		case t == "#":
			l = ""
		case strings.HasPrefix(t, "##"):
			l = l[:len(l)-len(t)] + t[1:]
		case strings.HasPrefix(t, "# "):
			l = l[:len(l)-len(t)] + t[2:]
		}
		t = strings.TrimSpace(l)
		switch {
		case strings.HasPrefix(t, "#![") && strings.HasSuffix(t, "]"):
			l = ""
		case strings.HasPrefix(t, "extern crate ") || strings.HasPrefix(t, "#[macro_use] extern crate ") || strings.HasPrefix(t, "#[macro_use]extern crate "):
			// hoist the `extern crate ..;` (the rest of the line stays)
			if k := strings.IndexByte(t, ';'); k >= 0 {
				hoisted = append(hoisted, t[:k+1])
				l = t[k+1:]
			}
		}
		lines[i] = l
	}
	body = strings.Join(lines, "\n")
	return body, hoisted, fnMainRe.MatchString(body)
}

// docTestFile is the source of test i's module: blank lines up to the
// fence, then the wrapped code starting on the line after it.
func docTestFile(i int, dt *DocTest) (src string, hoisted []string) {
	body, hoisted, hasMain := docTestCode(dt.Code)
	var b strings.Builder
	b.WriteString(strings.Repeat("\n", dt.Line-1))
	fmt.Fprintf(&b, "#[allow(unused)] pub mod __rustgo_doctest_%d { use super::*; ", i)
	switch {
	case hasMain:
		b.WriteString("\n" + body + "\n}\n")
	case strings.HasSuffix(strings.TrimSpace(body), "(())"):
		b.WriteString("pub fn main() { fn _inner() -> ::core::result::Result<(), impl ::core::fmt::Debug> {\n" + body + "\n} _inner().unwrap() } }\n")
	default:
		b.WriteString("pub fn main() {\n" + body + "\n} }\n")
	}
	return b.String(), hoisted
}

// docTestCrate builds the crate running tests (each test's Index is its
// argument).
func docTestCrate(tests []*DocTest) (*ast.Crate, error) {
	root := &ast.Crate{}
	seen := map[string]bool{}
	var hoist strings.Builder
	var disp strings.Builder
	disp.WriteString("fn main() {\n    let a: ::std::vec::Vec<::std::string::String> = ::std::env::args().collect();\n    match a.get(1).map(|s| s.as_str()).unwrap_or(\"\") {\n")
	for _, dt := range tests {
		src, hoisted := docTestFile(dt.Index, dt)
		for _, h := range hoisted {
			if !seen[h] {
				seen[h] = true
				hoist.WriteString(h + "\n")
			}
		}
		c, err := parser.ParseCrate(lexer.AddFile(dt.File, src), src)
		if err != nil {
			return nil, fmt.Errorf("%s: %v", dt.Name, err)
		}
		root.Items = append(root.Items, c.Items...)
		fmt.Fprintf(&disp, "        \"%d\" => ::std::process::exit(match ::std::rustgo_test::assert_test_result(crate::__rustgo_doctest_%d::main()) { Ok(()) => 0, Err(_) => 1 }),\n", dt.Index, dt.Index)
	}
	disp.WriteString("        _ => {}\n    }\n}\n")
	src := hoist.String() + disp.String()
	mc, err := parser.ParseCrate(lexer.AddFile("<doctest>", src), src)
	if err != nil {
		return nil, err
	}
	ast.Walk(mc, func(n any) bool {
		if pe, ok := n.(*ast.PathExpr); ok && pe.Path != nil && len(pe.Path.Segments) > 0 && pe.Path.Segments[0].Ident.Name == "crate" {
			for _, seg := range pe.Path.Segments {
				seg.Ident.Hyg = testHarnessMark
			}
		}
		return true
	})
	root.Items = append(mc.Items, root.Items...)
	return root, nil
}
