package expand

import (
	"fmt"
	"strconv"
	"strings"
	"unicode"

	"rustgo/compiler/ast"
	"rustgo/compiler/lexer"
)

// ---------------------------------------------------------------------------
// Format string parsing (mirrors rustc_parse_format)

// FmtArgKind says how a placeholder names its argument.
type FmtArgKind int

const (
	FmtArgIndex FmtArgKind = iota // {0}, or an implicit {} (Implicit = true)
	FmtArgName                    // {name}
)

// FmtArgRef references a format argument.
type FmtArgRef struct {
	Kind     FmtArgKind
	Index    int
	Name     string
	Implicit bool // position assigned implicitly ({} / {:..})
}

// FmtCountKind is the kind of a width/precision.
type FmtCountKind int

const (
	FmtCountNone  FmtCountKind = iota
	FmtCountIs                 // literal number
	FmtCountParam              // N$ / name$ / .* (Ref says which argument)
)

// FmtCount is a width or precision.
type FmtCount struct {
	Kind FmtCountKind
	N    int
	Ref  FmtArgRef
}

// FmtPlaceholder is one parsed `{...}`.
type FmtPlaceholder struct {
	Arg       FmtArgRef
	Fill      rune // 0 if unspecified
	Align     byte // 0, '<', '>', '^'
	Plus      bool
	Minus     bool
	Alternate bool
	ZeroPad   bool
	Width     FmtCount
	Prec      FmtCount
	Trait     string // Display, Debug, LowerHex, ...
	DebugHex  byte   // 'x' / 'X' for {:x?} / {:X?}
}

// FmtPiece is either a literal (Ph == nil) or a placeholder.
type FmtPiece struct {
	Lit string
	Ph  *FmtPlaceholder
}

// FmtError is a format string error with a byte offset into the string.
type FmtError struct {
	Off int
	Msg string
}

func (e *FmtError) Error() string { return "invalid format string: " + e.Msg }

var fmtTraits = map[string]string{
	"":   "Display",
	"?":  "Debug",
	"x?": "Debug",
	"X?": "Debug",
	"x":  "LowerHex",
	"X":  "UpperHex",
	"o":  "Octal",
	"b":  "Binary",
	"e":  "LowerExp",
	"E":  "UpperExp",
	"p":  "Pointer",
}

type fmtParser struct {
	s      []rune
	i      int
	curarg int
}

func isIdentStart(r rune) bool { return r == '_' || unicode.IsLetter(r) }
func isIdentCont(r rune) bool  { return r == '_' || unicode.IsLetter(r) || unicode.IsDigit(r) }

func (p *fmtParser) peek() rune {
	if p.i < len(p.s) {
		return p.s[p.i]
	}
	return -1
}

func (p *fmtParser) peekAt(k int) rune {
	if p.i+k < len(p.s) {
		return p.s[p.i+k]
	}
	return -1
}

func (p *fmtParser) errf(format string, args ...any) error {
	return &FmtError{Off: p.i, Msg: fmt.Sprintf(format, args...)}
}

func (p *fmtParser) integer() (int, bool) {
	start := p.i
	for p.i < len(p.s) && p.s[p.i] >= '0' && p.s[p.i] <= '9' {
		p.i++
	}
	if p.i == start {
		return 0, false
	}
	n, err := strconv.Atoi(string(p.s[start:p.i]))
	if err != nil {
		return 0, false
	}
	return n, true
}

func (p *fmtParser) word() string {
	if p.i >= len(p.s) || !isIdentStart(p.s[p.i]) {
		return ""
	}
	start := p.i
	for p.i < len(p.s) && isIdentCont(p.s[p.i]) {
		p.i++
	}
	return string(p.s[start:p.i])
}

// ParseFormatString parses a format string into literal pieces and
// placeholders. Implicit positions ({} and .*) are resolved to indices the
// way rustc does (the precision of `.*` is taken before the value).
func ParseFormatString(s string) ([]FmtPiece, error) {
	p := &fmtParser{s: []rune(s)}
	var pieces []FmtPiece
	var lit strings.Builder
	flush := func() {
		if lit.Len() > 0 {
			pieces = append(pieces, FmtPiece{Lit: lit.String()})
			lit.Reset()
		}
	}
	for p.i < len(p.s) {
		c := p.s[p.i]
		switch c {
		case '{':
			if p.peekAt(1) == '{' {
				lit.WriteRune('{')
				p.i += 2
				continue
			}
			p.i++
			ph, err := p.placeholder()
			if err != nil {
				return nil, err
			}
			flush()
			pieces = append(pieces, FmtPiece{Ph: ph})
		case '}':
			if p.peekAt(1) == '}' {
				lit.WriteRune('}')
				p.i += 2
				continue
			}
			return nil, p.errf("unmatched `}` found")
		default:
			lit.WriteRune(c)
			p.i++
		}
	}
	flush()
	return pieces, nil
}

func (p *fmtParser) placeholder() (*FmtPlaceholder, error) {
	ph := &FmtPlaceholder{}
	hasPos := false
	// Argument position.
	switch r := p.peek(); {
	case r >= '0' && r <= '9':
		n, _ := p.integer()
		ph.Arg = FmtArgRef{Kind: FmtArgIndex, Index: n}
		hasPos = true
	case isIdentStart(r):
		w := p.word()
		if w == "_" {
			return nil, p.errf("invalid argument name `_`")
		}
		ph.Arg = FmtArgRef{Kind: FmtArgName, Name: w}
		hasPos = true
	}
	trait := ""
	if p.peek() == ':' {
		p.i++
		var err error
		trait, err = p.spec(ph)
		if err != nil {
			return nil, err
		}
	}
	switch p.peek() {
	case '}':
		p.i++
	case -1:
		return nil, p.errf("expected `}` but string was terminated")
	default:
		return nil, p.errf("expected `}`, found `%c`", p.peek())
	}
	t, ok := fmtTraits[trait]
	if !ok {
		return nil, p.errf("unknown format trait `%s`", trait)
	}
	ph.Trait = t
	if trait == "x?" || trait == "X?" {
		ph.DebugHex = trait[0]
	}
	if !hasPos {
		ph.Arg = FmtArgRef{Kind: FmtArgIndex, Index: p.curarg, Implicit: true}
		p.curarg++
	}
	return ph, nil
}

// count parses `N`, `N$` or `name$`. ok is false when nothing matched (the
// position is restored).
func (p *fmtParser) count() (FmtCount, bool) {
	save := p.i
	if n, ok := p.integer(); ok {
		if p.peek() == '$' {
			p.i++
			return FmtCount{Kind: FmtCountParam, Ref: FmtArgRef{Kind: FmtArgIndex, Index: n}}, true
		}
		return FmtCount{Kind: FmtCountIs, N: n}, true
	}
	if w := p.word(); w != "" {
		if p.peek() == '$' {
			p.i++
			return FmtCount{Kind: FmtCountParam, Ref: FmtArgRef{Kind: FmtArgName, Name: w}}, true
		}
	}
	p.i = save
	return FmtCount{}, false
}

// spec parses [[fill]align][sign]['#']['0'][width]['.' precision]type and
// returns the type string.
func (p *fmtParser) spec(ph *FmtPlaceholder) (string, error) {
	isAlign := func(r rune) bool { return r == '<' || r == '>' || r == '^' }
	if r := p.peek(); r != -1 && r != '}' && isAlign(p.peekAt(1)) {
		ph.Fill = r
		ph.Align = byte(p.peekAt(1))
		p.i += 2
	} else if isAlign(r) {
		ph.Align = byte(r)
		p.i++
	}
	switch p.peek() {
	case '+':
		ph.Plus = true
		p.i++
	case '-':
		ph.Minus = true
		p.i++
	}
	if p.peek() == '#' {
		ph.Alternate = true
		p.i++
	}
	if p.peek() == '0' && p.peekAt(1) != '$' {
		ph.ZeroPad = true
		p.i++
	}
	if c, ok := p.count(); ok {
		ph.Width = c
	}
	if p.peek() == '.' {
		p.i++
		if p.peek() == '*' {
			p.i++
			ph.Prec = FmtCount{Kind: FmtCountParam, Ref: FmtArgRef{Kind: FmtArgIndex, Index: p.curarg, Implicit: true}}
			p.curarg++
		} else if c, ok := p.count(); ok {
			ph.Prec = c
		} else {
			return "", p.errf("expected precision after `.`")
		}
	}
	// type
	start := p.i
	if w := p.word(); w != "" {
		if p.peek() == '?' {
			p.i++
		}
	} else if p.peek() == '?' {
		p.i++
	}
	return string(p.s[start:p.i]), nil
}

// ---------------------------------------------------------------------------
// format_args! expansion

// fmtArg is one macro argument of format_args!.
type fmtArg struct {
	name     string // "" for positional
	tts      []ast.TokenTree
	span     lexer.Span
	used     bool
	captured bool // implicit capture of a variable
}

// formatInput is the parsed input of a format_args!-like macro.
type formatInput struct {
	str    string
	strSp  lexer.Span
	args   []*fmtArg
	byName map[string]int
	npos   int
}

// buildFormatArgs produces the token trees of the format_args! expansion.
func buildFormatArgs(sp lexer.Span, in *formatInput) ([]ast.TokenTree, error) {
	pieces, err := ParseFormatString(in.str)
	if err != nil {
		return nil, errAt(in.strSp, "%v", err)
	}
	if in.byName == nil {
		in.byName = map[string]int{}
	}
	resolve := func(r FmtArgRef) (int, error) {
		if r.Kind == FmtArgIndex {
			if r.Index >= len(in.args) || in.args[r.Index].captured {
				n := 0
				for _, a := range in.args {
					if !a.captured {
						n++
					}
				}
				if n == 0 {
					return 0, errAt(in.strSp, "%d positional argument%s in format string, but no arguments were given", r.Index+1, plural(r.Index+1))
				}
				return 0, errAt(in.strSp, "invalid reference to positional argument %d (there %s %d argument%s)", r.Index, isAre(n), n, plural(n))
			}
			in.args[r.Index].used = true
			return r.Index, nil
		}
		if i, ok := in.byName[r.Name]; ok {
			in.args[i].used = true
			return i, nil
		}
		// implicit capture
		a := &fmtArg{tts: []ast.TokenTree{identTok(r.Name, synthSpan(in.strSp))}, span: in.strSp, used: true, captured: true}
		in.args = append(in.args, a)
		in.byName[r.Name] = len(in.args) - 1
		return len(in.args) - 1, nil
	}
	type phRes struct {
		ph               *FmtPlaceholder
		arg, width, prec int
	}
	var res []phRes
	for _, pc := range pieces {
		if pc.Ph == nil {
			continue
		}
		r := phRes{ph: pc.Ph, width: -1, prec: -1}
		// rustc resolves width/precision args before the value.
		if pc.Ph.Width.Kind == FmtCountParam {
			if r.width, err = resolve(pc.Ph.Width.Ref); err != nil {
				return nil, err
			}
		}
		if pc.Ph.Prec.Kind == FmtCountParam {
			if r.prec, err = resolve(pc.Ph.Prec.Ref); err != nil {
				return nil, err
			}
		}
		if r.arg, err = resolve(pc.Ph.Arg); err != nil {
			return nil, err
		}
		res = append(res, r)
	}
	for _, a := range in.args {
		if !a.used {
			if a.name != "" {
				return nil, errAt(a.span, "named argument never used")
			}
			return nil, errAt(a.span, "argument never used")
		}
	}
	if len(res) == 0 {
		lit := ""
		for _, pc := range pieces {
			lit += pc.Lit
		}
		return tmpl(sp, "::std::fmt::Arguments::new_const("+rustStr(lit)+")")
	}

	var argTTs [][]ast.TokenTree
	var b strings.Builder
	b.WriteString("::std::fmt::Arguments::new({ let mut __fmt = ::std::fmt::Formatter::new(); match (")
	for i, a := range in.args {
		argTTs = append(argTTs, parenWrap(a.tts))
		fmt.Fprintf(&b, "&$%d, ", i)
	}
	b.WriteString(") { (")
	for i := range in.args {
		fmt.Fprintf(&b, "__arg%d, ", i)
	}
	b.WriteString(") => { ")
	k := 0
	for _, pc := range pieces {
		if pc.Ph == nil {
			if pc.Lit != "" {
				fmt.Fprintf(&b, "__fmt.write_str(%s); ", rustStr(pc.Lit))
			}
			continue
		}
		r := res[k]
		k++
		ph := r.ph
		fill := ph.Fill
		if fill == 0 {
			fill = ' '
		}
		align := 0
		switch ph.Align {
		case '<':
			align = 1
		case '>':
			align = 2
		case '^':
			align = 3
		}
		flags := 0
		if ph.Plus {
			flags |= 1
		}
		if ph.Minus {
			flags |= 2
		}
		if ph.Alternate {
			flags |= 4
		}
		if ph.ZeroPad {
			flags |= 8
		}
		switch ph.DebugHex {
		case 'x':
			flags |= 16
		case 'X':
			flags |= 32
		}
		count := func(c FmtCount, idx int) string {
			switch c.Kind {
			case FmtCountIs:
				return fmt.Sprintf("::std::option::Option::Some(%dusize)", c.N)
			case FmtCountParam:
				return fmt.Sprintf("::std::option::Option::Some(*__arg%d)", idx)
			}
			return "::std::option::Option::None"
		}
		fmt.Fprintf(&b, "__fmt.set_spec(%s, %du8, %du32, %s, %s); ", rustChar(fill), align, flags,
			count(ph.Width, r.width), count(ph.Prec, r.prec))
		fmt.Fprintf(&b, "let _ = ::std::fmt::%s::fmt(__arg%d, &mut __fmt); ", ph.Trait, r.arg)
	}
	b.WriteString("} }; __fmt.into_string() })")
	return tmpl(sp, b.String(), argTTs...)
}

func plural(n int) string {
	if n == 1 {
		return ""
	}
	return "s"
}

func isAre(n int) string {
	if n == 1 {
		return "is"
	}
	return "are"
}
