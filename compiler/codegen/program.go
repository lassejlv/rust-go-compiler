package codegen

import (
	"regexp"
	"strings"
	"sync"
)

// chunkBuf is a strings.Builder that records the end offset of every write:
// each write to Gen.fns is one complete top-level function definition, so
// the recorded offsets are the places where the function section can be
// split into translation units.
type chunkBuf struct {
	strings.Builder
	ends []int
}

func (c *chunkBuf) Write(p []byte) (int, error) {
	n, err := c.Builder.Write(p)
	c.ends = append(c.ends, c.Len())
	return n, err
}

func (c *chunkBuf) WriteString(s string) (int, error) {
	n, err := c.Builder.WriteString(s)
	c.ends = append(c.ends, c.Len())
	return n, err
}

// Program is a generated C program. Source renders it as a single
// translation unit (every generated function and global `static`); Units
// splits it into several translation units that are compiled separately and
// linked together.
type Program struct {
	types  string // typedefs and struct definitions
	protos string // function prototypes (and vtable declarations)
	data   string // global data definitions (`static DECL[ = INIT];` lines)
	fns    string // function definitions
	fnEnds []int  // end offsets of the function definitions in fns
	main   string // int main(...)
	// UsesAsm: the program contains asm! (Rust's Intel-syntax templates:
	// compile with -masm=intel)
	UsesAsm bool
}

// Source returns the program as one translation unit.
func (p *Program) Source() string {
	var b strings.Builder
	b.Grow(len(runtimeH) + len(coroH) + len(p.types) + len(p.protos) + len(p.data) + len(p.fns) + len(p.main) + 128)
	b.WriteString(runtimeH)
	b.WriteString(coroH)
	b.WriteString("\n/* ---- types ---- */\n")
	b.WriteString(p.types)
	b.WriteString("\n/* ---- prototypes ---- */\n")
	b.WriteString(p.protos)
	b.WriteString(p.data)
	b.WriteString("\n/* ---- functions ---- */\n")
	b.WriteString(p.fns)
	b.WriteString(p.main)
	return b.String()
}

// FnBytes is the size of the function definitions (the bulk of the work of
// the C compiler).
func (p *Program) FnBytes() int { return len(p.fns) }

// Units splits the program into at most n translation units of similar
// size. Every unit includes a common header (runtime, types, the prototypes
// of the functions used across units and `extern` declarations of the
// global data); unit 0 also defines the data, the runtime's globals and
// main. A generated function referenced only from the unit defining it
// stays `static` there (so that the C compiler may still inline it and drop
// its body); the others get external linkage, as does the global data.
// When n <= 1, or the program cannot be split safely, it returns the single
// unit Source().
func (p *Program) Units(n int) []string {
	if n > len(p.fnEnds) {
		n = len(p.fnEnds)
	}
	if n <= 1 || !RuntimeSplittable() {
		return []string{p.Source()}
	}
	single := []string{p.Source()}

	// prototypes of generated functions by name (without `static`); other
	// prototype lines (foreign functions, vtables) go to the header
	var hdrProtos strings.Builder
	protoOf := map[string]string{}
	var protoOrder []string
	for _, ln := range lines(p.protos) {
		switch {
		case strings.HasPrefix(ln, "extern "):
			hdrProtos.WriteString(ln)
		case !strings.HasPrefix(ln, "static ") || !strings.HasSuffix(ln, ";"):
			return single
		case strings.HasSuffix(ln, ");"):
			name := cFnName(ln)
			if name == "" {
				return single
			}
			if _, dup := protoOf[name]; !dup {
				protoOrder = append(protoOrder, name)
			}
			protoOf[name] = ln[len("static "):]
			continue
		default:
			hdrProtos.WriteString("extern ")
			hdrProtos.WriteString(ln[len("static "):])
		}
		hdrProtos.WriteByte('\n')
	}
	// data: `static DECL = INIT;` / `static DECL;` -> `extern DECL;` in the
	// header, `DECL = INIT;` in unit 0
	var defs strings.Builder
	for _, ln := range lines(p.data) {
		if !strings.HasPrefix(ln, "static ") || !strings.HasSuffix(ln, ";") {
			return single
		}
		rest := ln[len("static "):]
		decl := strings.TrimSuffix(rest, ";")
		if i := strings.Index(rest, " = "); i >= 0 {
			decl = rest[:i]
		}
		if strings.Contains(decl, ";") {
			return single
		}
		hdrProtos.WriteString("extern " + decl + ";\n")
		defs.WriteString(rest)
		defs.WriteByte('\n')
	}

	// split the function definitions (at definition boundaries) into n
	// runs of about the same size
	type fnDef struct {
		name, text string // text without the leading `static `
	}
	parts := make([][]fnDef, n)
	defUnit := map[string]int{}
	total := len(p.fns)
	start, u := 0, 0
	for _, end := range p.fnEnds {
		if end == start {
			continue
		}
		f := p.fns[start:end]
		name := cFnName(f)
		if !strings.HasPrefix(f, "static ") || name == "" {
			return single
		}
		if _, ok := protoOf[name]; !ok {
			return single
		}
		parts[u] = append(parts[u], fnDef{name, f[len("static "):]})
		defUnit[name] = u
		start = end
		// move on once this unit holds its share
		if u < n-1 && start >= total*(u+1)/n {
			u++
		}
	}
	if start != total {
		return single
	}

	// a function is exported when another unit refers to it (unit 0 also
	// holds the data, whose initializers refer to functions, and main)
	exported := map[string]bool{}
	mark := func(unit int, text string) {
		forEachIdent(text, func(id string) {
			if du, ok := defUnit[id]; ok && du != unit {
				exported[id] = true
			}
		})
	}
	for i, fs := range parts {
		for _, f := range fs {
			mark(i, f.text)
		}
	}
	mark(0, p.data)
	mark(0, p.main)
	for _, name := range protoOrder {
		if _, defined := defUnit[name]; !defined {
			exported[name] = true // (not expected: every prototype has a body)
		}
	}

	var hdr strings.Builder
	hdr.WriteString(runtimeH)
	hdr.WriteString(coroH)
	hdr.WriteString("\n/* ---- types ---- */\n")
	hdr.WriteString(p.types)
	hdr.WriteString("\n/* ---- prototypes ---- */\n")
	for _, name := range protoOrder {
		if exported[name] {
			hdr.WriteString(protoOf[name])
			hdr.WriteByte('\n')
		}
	}
	hdr.WriteString(hdrProtos.String())
	header := hdr.String()

	out := make([]string, n)
	for i, fs := range parts {
		var b strings.Builder
		b.WriteString("#define RG_MULTI_TU 1\n")
		if i == 0 {
			b.WriteString("#define RG_DEFINE_GLOBALS 1\n")
		}
		b.WriteString(header)
		// this unit's own functions
		for _, f := range fs {
			if !exported[f.name] {
				b.WriteString("static ")
				b.WriteString(protoOf[f.name])
				b.WriteByte('\n')
			}
		}
		if i == 0 {
			b.WriteString(defs.String())
		}
		b.WriteString("\n/* ---- functions ---- */\n")
		for _, f := range fs {
			if !exported[f.name] {
				b.WriteString("static ")
			}
			b.WriteString(f.text)
		}
		if i == 0 {
			b.WriteString(p.main)
		}
		out[i] = b.String()
	}
	return out
}

// cFnName returns the name of the function declared or defined by a
// generated `static T name(...)` line: the identifier before the first `(`.
func cFnName(s string) string {
	i := strings.IndexByte(s, '(')
	if i < 0 {
		return ""
	}
	j := i
	for j > 0 && isIdentByte(s[j-1]) {
		j--
	}
	return s[j:i]
}

func isIdentByte(c byte) bool {
	return c == '_' || c >= 'a' && c <= 'z' || c >= 'A' && c <= 'Z' || c >= '0' && c <= '9'
}

// forEachIdent calls f with every identifier-like token of C text (also
// those inside literals and comments: callers only over-approximate).
func forEachIdent(s string, f func(string)) {
	for i := 0; i < len(s); {
		c := s[i]
		if !isIdentByte(c) {
			i++
			continue
		}
		j := i + 1
		for j < len(s) && isIdentByte(s[j]) {
			j++
		}
		if c < '0' || c > '9' {
			f(s[i:j])
		}
		i = j
	}
}

func lines(s string) []string {
	s = strings.TrimSuffix(s, "\n")
	if s == "" {
		return nil
	}
	return strings.Split(s, "\n")
}

var (
	splittableOnce sync.Once
	splittable     bool
	splitProblem   string
)

// staticRe matches a `static` storage class at the start of a line (after
// attributes) or after `{`/`;` (a function-local static).
var staticRe = regexp.MustCompile(`(^\s*(__attribute__\(\(.*?\)\)\s*)*|[{;]\s*)static\s+(inline\s+)?`)

// RuntimeSplittable reports whether the runtime (runtime.h, coro.h) can be
// compiled as part of several translation units: its mutable globals must
// be declared through RG_GLOBAL (one definition program-wide) rather than
// `static` (one private copy per unit), and no runtime function may keep a
// function-local static variable.
func RuntimeSplittable() bool {
	splittableOnce.Do(func() {
		splitProblem = runtimeSplitProblem(runtimeH + "\n" + coroH)
		splittable = splitProblem == ""
	})
	return splittable
}

// runtimeSplitProblem returns the first line of src that declares static
// mutable data, or "".
func runtimeSplitProblem(src string) string {
	for _, ln := range strings.Split(src, "\n") {
		for _, m := range staticRe.FindAllStringIndex(ln, -1) {
			rest := ln[m[1]:]
			if strings.HasPrefix(rest, "const ") {
				continue // read-only data: a copy per unit is harmless
			}
			// a function declaration/definition: `(` before any of `=;[`
			i := strings.IndexAny(rest, "(=;[")
			indented := m[0] == 0 && len(ln) > 0 && (ln[0] == ' ' || ln[0] == '\t')
			if i >= 0 && rest[i] == '(' && m[0] == 0 && !indented {
				continue
			}
			return strings.TrimSpace(ln)
		}
	}
	return ""
}
