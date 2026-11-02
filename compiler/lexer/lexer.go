package lexer

import (
	"fmt"
	"strings"
	"unicode"
	"unicode/utf8"
)

// Error is a lexical error with a position.
type Error struct {
	Pos Pos
	Msg string
}

func (e *Error) Error() string { return fmt.Sprintf("%s: %s", e.Pos, e.Msg) }

type lx struct {
	file int
	src  string
	off  int
	line int
	col  int
	toks []Token
}

// Tokenize converts src into tokens. The result always ends with an EOF
// token. Comments (including doc comments) are skipped.
func Tokenize(file int, src string) ([]Token, error) {
	l := &lx{file: file, src: src, line: 1, col: 1}
	l.toks = make([]Token, 0, len(src)/5+16) // ~1 token per 5 bytes of source
	// Byte-order mark.
	if strings.HasPrefix(l.src, "\xEF\xBB\xBF") {
		l.off = 3
	}
	l.skipShebang()
	for {
		if err := l.skipTrivia(); err != nil {
			return nil, err
		}
		if l.off >= len(l.src) {
			p := l.pos()
			l.toks = append(l.toks, Token{Kind: EOF, Span: Span{p, p}})
			break
		}
		if err := l.lexToken(); err != nil {
			return nil, err
		}
	}
	for i := 0; i+1 < len(l.toks); i++ {
		if l.toks[i+1].Kind != EOF && l.toks[i].Span.Hi.Off == l.toks[i+1].Span.Lo.Off {
			l.toks[i].JointNext = true
		}
	}
	return l.toks, nil
}

func (l *lx) pos() Pos { return Pos{File: l.file, Off: l.off, Line: l.line, Col: l.col} }

func (l *lx) errf(p Pos, format string, args ...any) error {
	return &Error{Pos: p, Msg: fmt.Sprintf(format, args...)}
}

// peekAt returns the rune at byte offset off+delta (in bytes) and its size.
func (l *lx) runeAt(off int) (rune, int) {
	if off >= len(l.src) {
		return -1, 0
	}
	c := l.src[off]
	if c < utf8.RuneSelf {
		return rune(c), 1
	}
	return utf8.DecodeRuneInString(l.src[off:])
}

func (l *lx) cur() rune { r, _ := l.runeAt(l.off); return r }

// peek returns the n-th rune after the current one (n=1 is the next rune).
func (l *lx) peek(n int) rune {
	off := l.off
	for i := 0; i < n; i++ {
		_, sz := l.runeAt(off)
		if sz == 0 {
			return -1
		}
		off += sz
	}
	r, _ := l.runeAt(off)
	return r
}

func (l *lx) advance() rune {
	r, sz := l.runeAt(l.off)
	if sz == 0 {
		return -1
	}
	l.off += sz
	if r == '\n' {
		l.line++
		l.col = 1
	} else {
		l.col++
	}
	return r
}

func (l *lx) skipShebang() {
	rest := l.src[l.off:]
	if !strings.HasPrefix(rest, "#!") {
		return
	}
	// `#![` (possibly with whitespace/comments in between) is an inner attribute.
	after := strings.TrimLeft(rest[2:], " \t\r\n")
	if strings.HasPrefix(after, "[") {
		return
	}
	for l.off < len(l.src) && l.src[l.off] != '\n' {
		l.advance()
	}
}

func isWhite(r rune) bool {
	switch r {
	case ' ', '\t', '\n', '\r', '\v', '\f', 0x85, 0x200E, 0x200F, 0x2028, 0x2029:
		return true
	}
	return false
}

func (l *lx) skipTrivia() error {
	for l.off < len(l.src) {
		r := l.cur()
		if isWhite(r) {
			l.advance()
			continue
		}
		if r == '/' && l.peek(1) == '/' {
			for l.off < len(l.src) && l.src[l.off] != '\n' {
				l.advance()
			}
			continue
		}
		if r == '/' && l.peek(1) == '*' {
			start := l.pos()
			l.advance()
			l.advance()
			depth := 1
			for depth > 0 {
				if l.off >= len(l.src) {
					return l.errf(start, "unterminated block comment")
				}
				c := l.cur()
				if c == '/' && l.peek(1) == '*' {
					l.advance()
					l.advance()
					depth++
				} else if c == '*' && l.peek(1) == '/' {
					l.advance()
					l.advance()
					depth--
				} else {
					l.advance()
				}
			}
			continue
		}
		break
	}
	return nil
}

// IsIdentStart reports whether r can start an identifier.
func IsIdentStart(r rune) bool {
	return r == '_' || (r >= 'a' && r <= 'z') || (r >= 'A' && r <= 'Z') || (r >= 0x80 && unicode.IsLetter(r))
}

// IsIdentContinue reports whether r can continue an identifier.
func IsIdentContinue(r rune) bool {
	return r == '_' || (r >= 'a' && r <= 'z') || (r >= 'A' && r <= 'Z') || (r >= '0' && r <= '9') ||
		(r >= 0x80 && (unicode.IsLetter(r) || unicode.IsDigit(r) || unicode.Is(unicode.Mn, r) || unicode.Is(unicode.Mc, r) || unicode.Is(unicode.Pc, r)))
}

func (l *lx) emit(kind Kind, text string, start Pos) *Token {
	l.toks = append(l.toks, Token{Kind: kind, Text: text, Span: Span{start, l.pos()}})
	return &l.toks[len(l.toks)-1]
}

func (l *lx) readIdent() string {
	s := l.off
	for l.off < len(l.src) && IsIdentContinue(l.cur()) {
		l.advance()
	}
	return l.src[s:l.off]
}

func (l *lx) lexToken() error {
	start := l.pos()
	r := l.cur()
	switch {
	case r == 'r' && l.peek(1) == '#' && IsIdentStart(l.peek(2)):
		l.advance()
		l.advance()
		name := l.readIdent()
		t := l.emit(Ident, name, start)
		t.Raw = true
		return nil
	case r == 'r' && (l.peek(1) == '"' || (l.peek(1) == '#' && (l.peek(2) == '"' || l.peek(2) == '#'))):
		l.advance()
		return l.lexRawStr(Str, start)
	case r == 'b' && l.peek(1) == 'r' && (l.peek(2) == '"' || l.peek(2) == '#'):
		l.advance()
		l.advance()
		return l.lexRawStr(ByteStr, start)
	case r == 'c' && l.peek(1) == 'r' && (l.peek(2) == '"' || l.peek(2) == '#'):
		l.advance()
		l.advance()
		return l.lexRawStr(CStr, start)
	case r == 'b' && l.peek(1) == '"':
		l.advance()
		return l.lexStr(ByteStr, start)
	case r == 'c' && l.peek(1) == '"':
		l.advance()
		return l.lexStr(CStr, start)
	case r == 'b' && l.peek(1) == '\'':
		l.advance()
		return l.lexChar(Byte, start)
	case IsIdentStart(r):
		name := l.readIdent()
		l.emit(Ident, name, start)
		return nil
	case r >= '0' && r <= '9':
		return l.lexNumber(start)
	case r == '\'':
		return l.lexQuote(start)
	case r == '"':
		return l.lexStr(Str, start)
	}
	return l.lexPunct(start)
}

var puncts3 = []string{"<<=", ">>=", "...", "..="}
var puncts2 = []string{"::", "->", "=>", "==", "!=", "<=", ">=", "&&", "||", "+=", "-=", "*=", "/=", "%=", "^=", "&=", "|=", "<<", ">>", ".."}

const puncts1 = "+-*/%^!&|=<>.,;:()[]{}#$?@~"

func (l *lx) lexPunct(start Pos) error {
	rest := l.src[l.off:]
	for _, p := range puncts3 {
		if strings.HasPrefix(rest, p) {
			for range p {
				l.advance()
			}
			l.emit(Punct, p, start)
			return nil
		}
	}
	for _, p := range puncts2 {
		if strings.HasPrefix(rest, p) {
			l.advance()
			l.advance()
			l.emit(Punct, p, start)
			return nil
		}
	}
	if strings.IndexByte(puncts1, rest[0]) >= 0 {
		l.advance()
		l.emit(Punct, rest[:1], start)
		return nil
	}
	r := l.cur()
	return l.errf(start, "unknown start of token: %q", r)
}

func isDigitBase(r rune, base int) bool {
	switch base {
	case 2:
		return r == '0' || r == '1'
	case 8:
		return r >= '0' && r <= '7'
	case 16:
		return (r >= '0' && r <= '9') || (r >= 'a' && r <= 'f') || (r >= 'A' && r <= 'F')
	}
	return r >= '0' && r <= '9'
}

func (l *lx) lexNumber(start Pos) error {
	afterDot := len(l.toks) > 0 && l.toks[len(l.toks)-1].Kind == Punct && l.toks[len(l.toks)-1].Text == "."
	s := l.off
	base := 10
	if l.cur() == '0' {
		switch l.peek(1) {
		case 'x':
			base = 16
		case 'o':
			base = 8
		case 'b':
			base = 2
		}
	}
	isFloat := false
	if base != 10 {
		l.advance()
		l.advance()
		for l.off < len(l.src) {
			c := l.cur()
			if c == '_' || isDigitBase(c, base) || (base != 16 && c >= '0' && c <= '9') {
				l.advance()
				continue
			}
			break
		}
	} else {
		for l.off < len(l.src) && (l.cur() == '_' || (l.cur() >= '0' && l.cur() <= '9')) {
			l.advance()
		}
		if !afterDot {
			if l.cur() == '.' && l.peek(1) != '.' && !IsIdentStart(l.peek(1)) {
				l.advance()
				isFloat = true
				if l.cur() >= '0' && l.cur() <= '9' {
					for l.off < len(l.src) && (l.cur() == '_' || (l.cur() >= '0' && l.cur() <= '9')) {
						l.advance()
					}
				}
			}
			if c := l.cur(); c == 'e' || c == 'E' {
				n := l.peek(1)
				k := 1
				if n == '+' || n == '-' {
					n = l.peek(2)
					k = 2
				}
				for n == '_' {
					k++
					n = l.peek(k)
				}
				if n >= '0' && n <= '9' {
					isFloat = true
					l.advance()
					if l.cur() == '+' || l.cur() == '-' {
						l.advance()
					}
					for l.off < len(l.src) && (l.cur() == '_' || (l.cur() >= '0' && l.cur() <= '9')) {
						l.advance()
					}
				}
			}
		}
	}
	text := l.src[s:l.off]
	suffix := ""
	if IsIdentStart(l.cur()) {
		suffix = l.readIdent()
	}
	if suffix == "f32" || suffix == "f64" {
		if base != 10 {
			return l.errf(start, "binary/octal/hex float literal is not supported")
		}
		isFloat = true
	}
	kind := Int
	if isFloat {
		kind = Float
	}
	t := l.emit(kind, text, start)
	t.Suffix = suffix
	return nil
}

// lexQuote lexes a char literal or a lifetime/label.
func (l *lx) lexQuote(start Pos) error {
	n := l.peek(1)
	if n == '\\' {
		return l.lexChar(Char, start)
	}
	if IsIdentStart(n) || (n >= '0' && n <= '9') {
		if l.peek(2) == '\'' {
			return l.lexChar(Char, start)
		}
		l.advance() // '
		if l.cur() == 'r' && l.peek(1) == '#' && IsIdentStart(l.peek(2)) {
			l.advance()
			l.advance()
		}
		name := l.readIdent()
		l.emit(Lifetime, "'"+name, start)
		return nil
	}
	return l.lexChar(Char, start)
}

// lexChar lexes 'x' (Char) or, with the `b` prefix already consumed, b'x'.
func (l *lx) lexChar(kind Kind, start Pos) error {
	l.advance() // opening '
	isByte := kind == Byte
	var buf []byte
	c := l.cur()
	switch {
	case c == -1 || c == '\n':
		return l.errf(start, "unterminated character literal")
	case c == '\\':
		b, err := l.readEscape(isByte, false)
		if err != nil {
			return err
		}
		buf = b
	case c == '\'':
		return l.errf(start, "empty character literal")
	default:
		l.advance()
		if isByte {
			if c >= 0x80 {
				return l.errf(start, "non-ASCII character in byte literal")
			}
			buf = []byte{byte(c)}
		} else {
			buf = []byte(string(c))
		}
	}
	if l.cur() != '\'' {
		return l.errf(start, "unterminated character literal")
	}
	l.advance()
	t := l.emit(kind, string(buf), start)
	if IsIdentStart(l.cur()) {
		t.Suffix = l.readIdent()
		t.Span.Hi = l.pos()
	}
	return nil
}

// readEscape decodes an escape sequence starting at a backslash. For a
// line continuation (allowed in strings) it returns an empty slice.
func (l *lx) readEscape(isByte, inString bool) ([]byte, error) {
	start := l.pos()
	l.advance() // backslash
	c := l.advance()
	switch c {
	case 'n':
		return []byte{'\n'}, nil
	case 'r':
		return []byte{'\r'}, nil
	case 't':
		return []byte{'\t'}, nil
	case '\\':
		return []byte{'\\'}, nil
	case '0':
		return []byte{0}, nil
	case '\'':
		return []byte{'\''}, nil
	case '"':
		return []byte{'"'}, nil
	case 'x':
		h1, h2 := l.advance(), l.advance()
		if !isDigitBase(h1, 16) || !isDigitBase(h2, 16) {
			return nil, l.errf(start, "invalid \\x escape")
		}
		v := hexVal(h1)*16 + hexVal(h2)
		if !isByte && v > 0x7f {
			return nil, l.errf(start, "out of range hex escape (must be at most \\x7f)")
		}
		if isByte {
			return []byte{byte(v)}, nil
		}
		return []byte{byte(v)}, nil
	case 'u':
		if isByte {
			return nil, l.errf(start, "unicode escape in byte literal")
		}
		if l.advance() != '{' {
			return nil, l.errf(start, "invalid unicode escape")
		}
		v := 0
		digits := 0
		for {
			d := l.advance()
			if d == '}' {
				break
			}
			if d == '_' {
				continue
			}
			if !isDigitBase(d, 16) || digits >= 6 {
				return nil, l.errf(start, "invalid unicode escape")
			}
			v = v*16 + hexVal(d)
			digits++
		}
		if digits == 0 || v > 0x10FFFF || (v >= 0xD800 && v <= 0xDFFF) {
			return nil, l.errf(start, "invalid unicode escape")
		}
		return []byte(string(rune(v))), nil
	case '\n', '\r':
		if !inString {
			break
		}
		if c == '\r' && l.cur() == '\n' {
			l.advance()
		}
		for l.off < len(l.src) && isWhite(l.cur()) {
			l.advance()
		}
		return []byte{}, nil
	}
	return nil, l.errf(start, "unknown character escape: %q", c)
}

func hexVal(r rune) int {
	switch {
	case r >= '0' && r <= '9':
		return int(r - '0')
	case r >= 'a' && r <= 'f':
		return int(r-'a') + 10
	default:
		return int(r-'A') + 10
	}
}

// lexStr lexes "..." (the b/c prefix, if any, is already consumed).
func (l *lx) lexStr(kind Kind, start Pos) error {
	l.advance() // "
	isByte := kind == ByteStr
	var buf []byte
	for {
		c := l.cur()
		switch c {
		case -1:
			return l.errf(start, "unterminated string literal")
		case '"':
			l.advance()
			t := l.emit(kind, string(buf), start)
			if IsIdentStart(l.cur()) {
				t.Suffix = l.readIdent()
				t.Span.Hi = l.pos()
			}
			return nil
		case '\\':
			b, err := l.readEscape(isByte, true)
			if err != nil {
				return err
			}
			buf = append(buf, b...)
		case '\r':
			l.advance()
			if l.cur() != '\n' {
				buf = append(buf, '\r')
			}
		default:
			l.advance()
			if isByte && c >= 0x80 {
				return l.errf(start, "non-ASCII character in byte string literal")
			}
			buf = append(buf, string(c)...)
		}
	}
}

// lexRawStr lexes r#"..."# with the r/br/cr prefix already consumed (the
// current char is '#' or '"').
func (l *lx) lexRawStr(kind Kind, start Pos) error {
	hashes := 0
	for l.cur() == '#' {
		hashes++
		l.advance()
	}
	if l.cur() != '"' {
		return l.errf(start, "expected '\"' in raw string literal")
	}
	l.advance()
	closer := "\"" + strings.Repeat("#", hashes)
	var sb strings.Builder
	for {
		if l.off >= len(l.src) {
			return l.errf(start, "unterminated raw string literal")
		}
		if strings.HasPrefix(l.src[l.off:], closer) {
			for range closer {
				l.advance()
			}
			break
		}
		c := l.advance()
		if c == '\r' && l.cur() == '\n' {
			continue
		}
		sb.WriteRune(c)
	}
	t := l.emit(kind, sb.String(), start)
	t.Raw = true
	if IsIdentStart(l.cur()) {
		t.Suffix = l.readIdent()
		t.Span.Hi = l.pos()
	}
	return nil
}
