package expand

// Procedural macros run in a separate "macro server" process: the
// proc-macro crate is compiled (by the driver) together with the
// `proc_macro` crate into an executable that serves expansion requests over
// stdin/stdout. Token streams cross the process boundary in a simple
// serialized form (see serializeTTs).

import (
	"bufio"
	"fmt"
	"io"
	"os"
	"os/exec"
	"strconv"
	"strings"

	"rustgo/compiler/ast"
	"rustgo/compiler/lexer"
	"rustgo/compiler/parser"
)

type ProcMacroKind int

const (
	PMDerive ProcMacroKind = iota
	PMAttr
	PMBang
)

// ProcMacro is one exported procedural macro.
type ProcMacro struct {
	Kind    ProcMacroKind
	Name    string   // macro name (derive name / attribute name / bang name)
	Fn      string   // implementing function in the proc-macro crate
	Helpers []string // derive helper attributes
	Server  *PMServer
}

// PMServer is a running (lazily started) macro server.
type PMServer struct {
	Path string
	cmd  *exec.Cmd
	in   io.WriteCloser
	out  *bufio.Reader
}

func (s *PMServer) start() error {
	if s.cmd != nil {
		return nil
	}
	cmd := exec.Command(s.Path)
	in, err := cmd.StdinPipe()
	if err != nil {
		return err
	}
	out, err := cmd.StdoutPipe()
	if err != nil {
		return err
	}
	cmd.Stderr = os.Stderr
	if err := cmd.Start(); err != nil {
		return err
	}
	s.cmd, s.in, s.out = cmd, in, bufio.NewReaderSize(out, 1<<16)
	return nil
}

// Close stops the server.
func (s *PMServer) Close() {
	if s.cmd != nil {
		s.in.Close()
		s.cmd.Wait()
		s.cmd = nil
	}
}

func writePayload(w io.Writer, b string) error {
	_, err := fmt.Fprintf(w, "%d\n%s", len(b), b)
	return err
}

func readPayload(r *bufio.Reader) (string, error) {
	line, err := r.ReadString('\n')
	if err != nil {
		return "", err
	}
	n, err := strconv.Atoi(strings.TrimSpace(line))
	if err != nil {
		return "", fmt.Errorf("bad payload length %q", line)
	}
	buf := make([]byte, n)
	if _, err := io.ReadFull(r, buf); err != nil {
		return "", err
	}
	return string(buf), nil
}

// Call invokes macro fn on the server.
func (s *PMServer) Call(kind ProcMacroKind, fn string, attr, input []ast.TokenTree, site lexer.Span) ([]ast.TokenTree, error) {
	if err := s.start(); err != nil {
		return nil, fmt.Errorf("starting proc-macro server: %v", err)
	}
	k := [...]string{"derive", "attr", "bang"}[kind]
	fmt.Fprintf(s.in, "REQ %s %s\n", k, fn)
	writePayload(s.in, serializeTTs(attr))
	writePayload(s.in, serializeTTs(input))
	for {
		line, err := s.out.ReadString('\n')
		if err != nil {
			return nil, fmt.Errorf("proc-macro server died: %v", err)
		}
		switch strings.TrimSpace(line) {
		case "LEX":
			text, err := readPayload(s.out)
			if err != nil {
				return nil, err
			}
			fid := lexer.AddFile("<proc-macro source>", text)
			tts, lerr := parser.TokenizeToTrees(fid, text)
			if lerr != nil {
				fmt.Fprintf(s.in, "ERR\n")
				writePayload(s.in, lerr.Error())
				continue
			}
			fmt.Fprintf(s.in, "OK\n")
			writePayload(s.in, serializeTTs(tts))
		case "OK":
			p, err := readPayload(s.out)
			if err != nil {
				return nil, err
			}
			return deserializeTTs(p, site)
		case "PANIC":
			p, _ := readPayload(s.out)
			return nil, fmt.Errorf("proc macro panicked: %s", p)
		default:
			return nil, fmt.Errorf("proc-macro server protocol error: %q", line)
		}
	}
}

// ---------------------------------------------------------------------------
// Serialization: one record per token: <tag><len>:<bytes>\n
//   ( [ { N  group open (len 0)      )  group close
//   I ident   R raw ident   P punct (char + 'J' or 'A')   L literal source

func serializeTTs(tts []ast.TokenTree) string {
	var b strings.Builder
	var rec func(tts []ast.TokenTree)
	put := func(tag byte, s string) {
		b.WriteByte(tag)
		b.WriteString(strconv.Itoa(len(s)))
		b.WriteByte(':')
		b.WriteString(s)
		b.WriteByte('\n')
	}
	rec = func(tts []ast.TokenTree) {
		for _, tt := range tts {
			if tt.Delim != 0 {
				put(tt.Delim, "")
				rec(tt.Inner)
				put(')', "")
				continue
			}
			t := tt.Tok
			switch t.Kind {
			case lexer.Ident:
				if t.Raw {
					put('R', t.Text)
				} else {
					put('I', t.Text)
				}
			case lexer.Lifetime:
				put('P', "'J")
				put('I', strings.TrimPrefix(t.Text, "'"))
			case lexer.Punct:
				// multi-char operators become joint single-char puncts
				for i, c := range t.Text {
					sp := "J"
					if i == len(t.Text)-1 && !t.JointNext {
						sp = "A"
					}
					put('P', string(c)+sp)
				}
			case lexer.EOF:
			default:
				put('L', literalSource(t))
			}
		}
	}
	rec(tts)
	return b.String()
}

// literalSource reconstructs Rust source for a literal token.
func literalSource(t lexer.Token) string {
	switch t.Kind {
	case lexer.Int, lexer.Float:
		return t.Text + t.Suffix
	case lexer.Str:
		return quoteRust(t.Text, '"') + t.Suffix
	case lexer.ByteStr:
		return "b" + quoteRust(t.Text, '"') + t.Suffix
	case lexer.CStr:
		return "c" + quoteRust(t.Text, '"') + t.Suffix
	case lexer.Char:
		return quoteRust(t.Text, '\'') + t.Suffix
	case lexer.Byte:
		return "b" + quoteRust(t.Text, '\'') + t.Suffix
	}
	return t.Text
}

func quoteRust(s string, q byte) string {
	var b strings.Builder
	b.WriteByte(q)
	for i := 0; i < len(s); i++ {
		c := s[i]
		switch {
		case c == q || c == '\\':
			b.WriteByte('\\')
			b.WriteByte(c)
		case c == '\n':
			b.WriteString(`\n`)
		case c == '\r':
			b.WriteString(`\r`)
		case c == '\t':
			b.WriteString(`\t`)
		case c == 0:
			b.WriteString(`\0`)
		case c < 32 || c == 127:
			fmt.Fprintf(&b, `\x%02x`, c)
		default:
			b.WriteByte(c)
		}
	}
	b.WriteByte(q)
	return b.String()
}

// deserializeTTs rebuilds token trees from the server's output; all tokens
// get the call-site span.
func deserializeTTs(s string, site lexer.Span) ([]ast.TokenTree, error) {
	var toks []lexer.Token
	i := 0
	for i < len(s) {
		tag := s[i]
		j := strings.IndexByte(s[i:], ':')
		if j < 0 {
			return nil, fmt.Errorf("bad token record")
		}
		n, err := strconv.Atoi(s[i+1 : i+j])
		if err != nil {
			return nil, fmt.Errorf("bad token record length")
		}
		body := s[i+j+1 : i+j+1+n]
		i = i + j + 1 + n + 1
		tok := lexer.Token{Span: site}
		switch tag {
		case '(', '[', '{':
			tok.Kind, tok.Text = lexer.Punct, string(tag)
		case 'N':
			continue // none-delimited groups are transparent
		case ')':
			// closing delimiter of the most recent open group
			tok.Kind = lexer.Punct
			tok.Text = closeFor(toks)
			if tok.Text == "" {
				continue
			}
		case 'I', 'R':
			tok.Kind, tok.Text, tok.Raw = lexer.Ident, body, tag == 'R'
		case 'P':
			rs := []rune(body)
			tok.Kind, tok.Text = lexer.Punct, string(rs[0])
			tok.JointNext = len(rs) > 1 && rs[1] == 'J'
		case 'L':
			lt, err := lexer.Tokenize(lexer.AddFile("<proc-macro literal>", body), body)
			if err != nil || len(lt) < 1 {
				return nil, fmt.Errorf("proc macro produced invalid literal %q", body)
			}
			tok = lt[0]
			if tok.Kind == lexer.Punct && tok.Text == "-" && len(lt) > 1 {
				// negative literal: emit '-' then the number
				tok.Span = site
				toks = append(toks, tok)
				tok = lt[1]
			}
			tok.Span = site
		default:
			return nil, fmt.Errorf("bad token tag %q", tag)
		}
		toks = append(toks, tok)
	}
	toks = mergePuncts(toks)
	toks = append(toks, lexer.Token{Kind: lexer.EOF, Span: site})
	return parser.TokenTrees(toks)
}

// closeFor finds the matching closing delimiter for the innermost open group.
func closeFor(toks []lexer.Token) string {
	depth := 0
	for k := len(toks) - 1; k >= 0; k-- {
		t := toks[k]
		if t.Kind != lexer.Punct {
			continue
		}
		switch t.Text {
		case ")", "]", "}":
			depth++
		case "(", "[", "{":
			if depth == 0 {
				return map[string]string{"(": ")", "[": "]", "{": "}"}[t.Text]
			}
			depth--
		}
	}
	return ""
}

var multiPuncts = []string{"<<=", ">>=", "...", "..=", "::", "->", "=>", "==", "!=", "<=", ">=", "&&", "||", "+=", "-=", "*=", "/=", "%=", "^=", "&=", "|=", "<<", ">>", ".."}

// mergePuncts joins joint single-char puncts into Rust operators and
// lifetimes ('a) as the lexer would produce them.
func mergePuncts(toks []lexer.Token) []lexer.Token {
	var out []lexer.Token
	for i := 0; i < len(toks); i++ {
		t := toks[i]
		if t.Kind == lexer.Punct && t.Text == "'" && i+1 < len(toks) && toks[i+1].Kind == lexer.Ident {
			out = append(out, lexer.Token{Kind: lexer.Lifetime, Text: "'" + toks[i+1].Text, Span: t.Span})
			i++
			continue
		}
		if t.Kind == lexer.Punct && len(t.Text) == 1 && t.JointNext {
			merged := false
			for _, op := range multiPuncts {
				if i+len(op) > len(toks) {
					continue
				}
				ok := true
				for k := 0; k < len(op); k++ {
					tk := toks[i+k]
					if tk.Kind != lexer.Punct || tk.Text != string(op[k]) || (k < len(op)-1 && !tk.JointNext) {
						ok = false
						break
					}
				}
				if ok {
					nt := t
					nt.Text = op
					nt.JointNext = toks[i+len(op)-1].JointNext
					out = append(out, nt)
					i += len(op) - 1
					merged = true
					break
				}
			}
			if merged {
				continue
			}
		}
		out = append(out, t)
	}
	return out
}

// itemTokens returns the token trees of an item including the given outer
// attributes.
func itemTokens(it *ast.Item, attrs []*ast.Attr) ([]ast.TokenTree, error) {
	var toks []lexer.Token
	for _, a := range attrs {
		sp := a.Span
		toks = append(toks, lexer.Token{Kind: lexer.Punct, Text: "#", Span: sp}, lexer.Token{Kind: lexer.Punct, Text: "[", Span: sp})
		for i, seg := range strings.Split(a.Path, "::") {
			if i > 0 {
				toks = append(toks, lexer.Token{Kind: lexer.Punct, Text: "::", Span: sp})
			}
			toks = append(toks, lexer.Token{Kind: lexer.Ident, Text: seg, Span: sp})
		}
		toks = append(toks, parser.FlattenTokenTrees(a.Tokens)...)
		toks = append(toks, lexer.Token{Kind: lexer.Punct, Text: "]", Span: sp})
	}
	if len(it.Tokens) == 0 {
		return nil, fmt.Errorf("%s: error: item has no tokens for proc macro input", it.Span.Lo)
	}
	toks = append(toks, it.Tokens...)
	toks = append(toks, lexer.Token{Kind: lexer.EOF, Span: it.Span})
	return parser.TokenTrees(toks)
}

// parseItemsFromTTs parses proc-macro output as items.
func parseItemsFromTTs(tts []ast.TokenTree) ([]*ast.Item, error) {
	var items []*ast.Item
	err := safeParse(func() error {
		var err error
		items, err = parser.NewFromTokenTrees(tts).ParseItems()
		return err
	})
	return items, err
}

func lastSeg(path string) string {
	if i := strings.LastIndex(path, "::"); i >= 0 {
		return path[i+2:]
	}
	return path
}

// procDerive runs a derive proc macro on an item.
func (e *Expander) procDerive(pm *ProcMacro, it *ast.Item, attrs []*ast.Attr, depth int) ([]*ast.Item, error) {
	input, err := itemTokens(it, attrs)
	if err != nil {
		return nil, err
	}
	out, err := pm.Server.Call(PMDerive, pm.Fn, nil, input, it.Span)
	if err != nil {
		return nil, fmt.Errorf("%s: error: in #[derive(%s)]: %v", it.Span.Lo, pm.Name, err)
	}
	items, err := parseItemsFromTTs(out)
	if err != nil {
		return nil, fmt.Errorf("%s: error: parsing output of #[derive(%s)]: %v", it.Span.Lo, pm.Name, err)
	}
	return e.expandItems(items, depth+1)
}

// procAttr runs an attribute proc macro if one of the item's attributes
// names one. ok=false if none applies.
func (e *Expander) procAttr(it *ast.Item, depth int) ([]*ast.Item, bool, error) {
	if len(e.ProcMacros) == 0 {
		return nil, false, nil
	}
	for i, a := range it.Attrs {
		pm := e.ProcMacros[lastSeg(a.Path)]
		if pm == nil || pm.Kind != PMAttr {
			continue
		}
		rest := append(append([]*ast.Attr{}, it.Attrs[:i]...), it.Attrs[i+1:]...)
		input, err := itemTokens(it, rest)
		if err != nil {
			return nil, true, err
		}
		var args []ast.TokenTree
		if len(a.Tokens) == 1 && a.Tokens[0].Delim == '(' {
			args = a.Tokens[0].Inner
		}
		out, err := pm.Server.Call(PMAttr, pm.Fn, args, input, a.Span)
		if err != nil {
			return nil, true, fmt.Errorf("%s: error: in #[%s]: %v", a.Span.Lo, a.Path, err)
		}
		items, err := parseItemsFromTTs(out)
		if err != nil {
			return nil, true, fmt.Errorf("%s: error: parsing output of #[%s]: %v", a.Span.Lo, a.Path, err)
		}
		res, err := e.expandItems(items, depth+1)
		return res, true, err
	}
	return nil, false, nil
}
