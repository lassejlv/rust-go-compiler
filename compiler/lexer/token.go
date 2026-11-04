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
	// Edition is the Rust edition of the crate the file belongs to
	// ("2015", "2018", "2021", "2024"; "" = 2021). As in rustc, where a
	// token's span decides the edition rules that apply to it, the edition
	// of a token (and of the code it is part of) is its file's.
	Edition string
}

// Files is the global source map.
var Files []*SourceFile

// CurEdition is the edition recorded for the files AddFile registers (the
// edition of the crate being loaded and expanded).
var CurEdition string

// AddFile registers a file and returns its index.
func AddFile(name, src string) int {
	Files = append(Files, &SourceFile{Name: name, Src: src, Edition: CurEdition})
	return len(Files) - 1
}

// FileEdition returns the edition of file idx ("2021" when unknown).
func FileEdition[I int | int32](idx I) string {
	if idx < 0 || int(idx) >= len(Files) || Files[idx].Edition == "" {
		return "2021"
	}
	return Files[idx].Edition
}

// Edition returns the edition of the file the span is in.
func (s Span) Edition() string { return FileEdition(s.Lo.File) }

// Before2018 reports whether the span is in a Rust 2015 file.
func (s Span) Before2018() bool { return s.Edition() == "2015" }

// Before2021 reports whether the span is in a Rust 2015 or 2018 file.
func (s Span) Before2021() bool { e := s.Edition(); return e == "2015" || e == "2018" }

// Since2024 reports whether the span is in a Rust 2024 (or later) file.
func (s Span) Since2024() bool { return s.Edition() >= "2024" }

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
	Hyg  uint32
	Kind Kind
	Raw  bool // raw identifier r#foo / raw string
	// Weak marks a Rust 2015 `async`/`await`/`dyn`/`try` the parser sees
	// as a raw identifier (see Is2015Ident); it is printed without `r#`.
	Weak  bool
	Inner bool // for DocComment: //! (inner) vs /// (outer)
	// JointNext is true when the next token immediately follows this one with
	// no whitespace (used for splitting/merging punctuation in macros).
	JointNext bool
	// Nt records how a token was substituted for a macro_rules fragment
	// (NtIdent, NtStart, NtEnd), for stringify!'s spacing. In a token
	// tree, a group's Tok.Nt holds the flags of the group.
	Nt uint8
}

// Token.Nt flags. rustc substitutes an `ident` fragment as an NtIdent
// token and most other fragments (expr, ty, path, ...) as a group with
// invisible delimiters; stringify! spaces them accordingly (`$f($x)` with
// `$x:ident` prints `f (x)`, `$t,` with `$t:ty` prints `Vec<u8>,`).
const (
	NtIdent uint8 = 1 << iota // substituted for an `ident` fragment
	NtStart                   // first tree of an invisible-group fragment
	NtEnd                     // last tree of an invisible-group fragment
	// NtTurbofish marks a `::` inserted before the generic arguments of a
	// substituted `path` fragment (`a::B<T>` used as `a::B::<T>`); it is
	// not part of the fragment's text (stringify!).
	NtTurbofish
)

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

// Keywords2018 are the keywords introduced by Rust 2018: plain identifiers
// in Rust 2015 code (`dyn` stays a contextual keyword in types there).
var Keywords2018 = map[string]bool{"async": true, "await": true, "dyn": true, "try": true}

// IsKeyword reports whether a (non-raw) identifier token is a keyword in
// the edition of its span.
func (t Token) IsKeyword() bool {
	return t.Kind == Ident && !t.Raw && Keywords[t.Text] && !t.Is2015Ident()
}

// Is2015Ident reports whether t is a non-raw `async`, `await`, `dyn` or
// `try` of Rust 2015 code, which is an identifier there.
func (t Token) Is2015Ident() bool {
	if t.Kind != Ident || t.Raw {
		return false
	}
	switch t.Text {
	case "async", "await", "dyn", "try":
		return t.Span.Before2018()
	}
	return false
}
