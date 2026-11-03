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
	// direct, if set, names a function taking (pieces, specs, args) that
	// consumes the formatted text directly (e.g. `::std::io::__print`),
	// called instead of building an `Arguments` value when the string has
	// placeholders; isDirect reports whether that call was emitted.
	direct   string
	isDirect bool
}

// phRes is a placeholder with its resolved argument indices (-1: none).
type phRes struct {
	ph               *FmtPlaceholder
	arg, width, prec int
}

// literalText returns the text a literal argument formats to with `{}`
// (string and integer literals, which rustc inlines into the format string).
func literalText(tts []ast.TokenTree) (string, bool) {
	if len(tts) != 1 || tts[0].Delim != 0 {
		return "", false
	}
	t := tts[0].Tok
	switch t.Kind {
	case lexer.Str:
		if t.Suffix != "" {
			return "", false
		}
		return t.Text, true
	case lexer.Int:
		switch t.Suffix {
		case "", "u8", "u16", "u32", "u64", "usize", "i8", "i16", "i32", "i64", "isize":
		default:
			return "", false
		}
		v, err := strconv.ParseUint(strings.ReplaceAll(t.Text, "_", ""), 0, 64)
		if err != nil {
			return "", false
		}
		return strconv.FormatUint(v, 10), true
	}
	return "", false
}

// inlineLiteralArgs inlines literal arguments of plain `{}` placeholders
// into the format string, like rustc (so that `panic!("{}", "x")` has a
// constant message and `Arguments::as_str` returns it); arguments left
// unused are dropped.
func inlineLiteralArgs(in *formatInput, pieces []FmtPiece, res []phRes) ([]FmtPiece, []phRes) {
	text := make([]string, len(res))
	inl := make([]bool, len(res))
	uses := make([]int, len(in.args))
	inlUses := make([]int, len(in.args))
	any := false
	for k, r := range res {
		uses[r.arg]++
		if r.width >= 0 {
			uses[r.width]++
		}
		if r.prec >= 0 {
			uses[r.prec]++
		}
		ph := r.ph
		if ph.Trait != "Display" || ph.Fill != 0 || ph.Align != 0 || ph.Plus || ph.Minus || ph.Alternate || ph.ZeroPad ||
			ph.Width.Kind != FmtCountNone || ph.Prec.Kind != FmtCountNone || in.args[r.arg].captured {
			continue
		}
		if t, ok := literalText(in.args[r.arg].tts); ok {
			text[k], inl[k] = t, true
			inlUses[r.arg]++
			any = true
		}
	}
	if !any {
		return pieces, res
	}
	var np []FmtPiece
	var nr []phRes
	k := 0
	for _, pc := range pieces {
		if pc.Ph == nil {
			np = append(np, pc)
			continue
		}
		if inl[k] {
			np = append(np, FmtPiece{Lit: text[k]})
		} else {
			np = append(np, pc)
			nr = append(nr, res[k])
		}
		k++
	}
	// drop the arguments that are no longer used
	newIdx := make([]int, len(in.args))
	var args []*fmtArg
	for i, a := range in.args {
		if uses[i] > 0 && uses[i] == inlUses[i] {
			newIdx[i] = -1
			continue
		}
		newIdx[i] = len(args)
		args = append(args, a)
	}
	in.args = args
	for i := range nr {
		nr[i].arg = newIdx[nr[i].arg]
		if nr[i].width >= 0 {
			nr[i].width = newIdx[nr[i].width]
		}
		if nr[i].prec >= 0 {
			nr[i].prec = newIdx[nr[i].prec]
		}
	}
	return np, nr
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
	pieces, res = inlineLiteralArgs(in, pieces, res)
	if len(res) == 0 {
		lit := ""
		for _, pc := range pieces {
			lit += pc.Lit
		}
		return tmpl(sp, "::std::fmt::Arguments::new_const("+rustStr(lit)+")")
	}

	// The formatting itself is done out of line by std's fmt::__format
	// (like rustc's fmt::write over `Arguments`): each format macro use is
	// a call with static pieces/specs and an array of `&dyn Trait` args,
	// which keeps the generated code small.
	var argTTs [][]ast.TokenTree
	for _, a := range in.args {
		argTTs = append(argTTs, parenWrap(a.tts))
	}
	// the arguments can be used in place when each is used once, in order
	inline := len(res) == len(in.args)
	for k, r := range res {
		if r.arg != k || r.width >= 0 || r.prec >= 0 {
			inline = false
		}
	}
	argRef := func(i int) string {
		if inline {
			return fmt.Sprintf("&$%d", i)
		}
		return fmt.Sprintf("__arg%d", i)
	}
	var pcs []string
	lit := ""
	for _, pc := range pieces {
		if pc.Ph == nil {
			lit += pc.Lit
			continue
		}
		pcs = append(pcs, rustStr(lit))
		lit = ""
	}
	if lit != "" {
		pcs = append(pcs, rustStr(lit))
	}
	var specs, args []string
	allDefault := true
	for _, r := range res {
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
				return fmt.Sprintf("::std::option::Option::Some(*%s)", argRef(idx))
			}
			return "::std::option::Option::None"
		}
		if fill != ' ' || align != 0 || flags != 0 || ph.Width.Kind != FmtCountNone || ph.Prec.Kind != FmtCountNone {
			allDefault = false
		}
		specs = append(specs, fmt.Sprintf("::std::fmt::__Spec { fill: %s, align: %du8, flags: %du32, width: %s, precision: %s }",
			rustChar(fill), align, flags, count(ph.Width, r.width), count(ph.Prec, r.prec)))
		// a generic constructor (like rustc's Argument::new_display), so
		// that the argument is not type-checked against `&dyn Trait`
		args = append(args, fmt.Sprintf("::std::fmt::__Arg::%s(%s)", argCtor[ph.Trait], argRef(r.arg)))
	}
	if allDefault {
		specs = nil
	}
	callee := "::std::fmt::__format_args"
	wrap := "("
	if in.direct != "" {
		callee, wrap = in.direct, "("
		in.isDirect = true
	}
	call := fmt.Sprintf("%s(&[%s], &[%s], &[%s])", callee, strings.Join(pcs, ", "), strings.Join(specs, ", "), strings.Join(args, ", "))
	if inline {
		return tmpl(sp, wrap+call+")", argTTs...)
	}
	var b strings.Builder
	b.WriteString(wrap + "match (")
	for i := range in.args {
		fmt.Fprintf(&b, "&$%d, ", i)
	}
	b.WriteString(") { (")
	for i := range in.args {
		fmt.Fprintf(&b, "__arg%d, ", i)
	}
	b.WriteString(") => " + call + " })")
	return tmpl(sp, b.String(), argTTs...)
}

// argCtor names the __Arg constructor of each formatting trait.
var argCtor = map[string]string{
	"Display":  "new_display",
	"Debug":    "new_debug",
	"LowerHex": "new_lower_hex",
	"UpperHex": "new_upper_hex",
	"Octal":    "new_octal",
	"Binary":   "new_binary",
	"LowerExp": "new_lower_exp",
	"UpperExp": "new_upper_exp",
	"Pointer":  "new_pointer",
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
