// Package lexer turns Rust source text into tokens (mirrors rustc_lexer +
// the token layer of rustc_parse).
package lexer

import (
	"fmt"
	"strings"
)

// Pos identifies a location in a source file.
type Pos struct {
	File int32 // index into the global SourceMap
	Off  int32 // byte offset
	Line int32 // 1-based
	Col  int32 // 1-based (in characters)
}

func (p Pos) String() string {
	return fmt.Sprintf("%s:%d:%d", FileName(p.File), p.Line, p.Col)
}

// Span is a half-open source range.
type Span struct {
	Lo, Hi Pos
}

func (s Span) String() string { return s.Lo.String() }

// SourceFile is a loaded source file.
type SourceFile struct {
	Name string
	Src  string
}

// Files is the global source map.
var Files []*SourceFile

// AddFile registers a file and returns its index.
func AddFile(name, src string) int {
	Files = append(Files, &SourceFile{Name: name, Src: src})
	return len(Files) - 1
}

// BaseDir is the directory relative display names are relative to ("" for
// the working directory).
var BaseDir string

// FilePath returns the path of file idx on disk (its display name resolved
// against BaseDir).
func FilePath(idx int) string {
	n := FileName(idx)
	if BaseDir != "" && !strings.HasPrefix(n, "/") && !strings.HasPrefix(n, "<") {
		return BaseDir + "/" + n
	}
	return n
}

// FileName returns the display name of file idx.
func FileName[I int | int32](idx I) string {
	if idx < 0 || int(idx) >= len(Files) {
		return "<unknown>"
	}
	return Files[idx].Name
}

// Kind is a token kind.
type Kind uint8

const (
	EOF        Kind = iota
	Ident           // identifiers and keywords (check Text); raw idents have Raw=true
	Lifetime        // 'a  (Text includes the leading quote)
	Int             // integer literal (Text = digits without suffix, Suffix separate)
	Float           // float literal
	Str             // "..." (Text = decoded value)
	ByteStr         // b"..."
	CStr            // c"..."
	Char            // 'x'  (Text = decoded value)
	Byte            // b'x'
	Punct           // operators and delimiters (Text = the operator)
	DocComment      // /// or //! (Text = content), kept for attrs; parser may skip
)

var kindNames = [...]string{"EOF", "Ident", "Lifetime", "Int", "Float", "Str", "ByteStr", "CStr", "Char", "Byte", "Punct", "DocComment"}

func (k Kind) String() string { return kindNames[k] }

// Token is a single lexical token.
type Token struct {
	Text   string // see Kind docs; for Punct the operator, for Ident the name
	Suffix string // literal suffix, e.g. "u8", "f64"
	Span   Span
	// Hyg is the macro_rules hygiene mark of an identifier introduced by a
	// macro body (0 for source text and macro arguments).
	Hyg   uint32
	Kind  Kind
	Raw   bool // raw identifier r#foo / raw string
	Inner bool // for DocComment: //! (inner) vs /// (outer)
	// JointNext is true when the next token immediately follows this one with
	// no whitespace (used for splitting/merging punctuation in macros).
	JointNext bool
}

func (t Token) String() string {
	switch t.Kind {
	case EOF:
		return "<eof>"
	case Str:
		return fmt.Sprintf("%q", t.Text)
	case Char:
		return fmt.Sprintf("'%s'", t.Text)
	case Int, Float:
		return t.Text + t.Suffix
	}
	return t.Text
}

// Is reports whether t is the punctuation or keyword/ident s.
func (t Token) Is(s string) bool {
	return (t.Kind == Punct || (t.Kind == Ident && !t.Raw)) && t.Text == s
}

// Keywords reserved in Rust 2021 (strict + reserved).
var Keywords = map[string]bool{
	"as": true, "break": true, "const": true, "continue": true, "crate": true,
	"else": true, "enum": true, "extern": true, "false": true, "fn": true,
	"for": true, "if": true, "impl": true, "in": true, "let": true, "loop": true,
	"match": true, "mod": true, "move": true, "mut": true, "pub": true,
	"ref": true, "return": true, "self": true, "Self": true, "static": true,
	"struct": true, "super": true, "trait": true, "true": true, "type": true,
	"unsafe": true, "use": true, "where": true, "while": true, "async": true,
	"await": true, "dyn": true,
	// reserved
	"abstract": true, "become": true, "box": true, "do": true, "final": true,
	"macro": true, "override": true, "priv": true, "typeof": true,
	"unsized": true, "virtual": true, "yield": true, "try": true,
}

// IsKeyword reports whether a (non-raw) identifier token is a keyword.
func (t Token) IsKeyword() bool { return t.Kind == Ident && !t.Raw && Keywords[t.Text] }
