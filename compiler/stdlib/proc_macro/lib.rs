//! The `proc_macro` crate for rustgo.
//!
//! Procedural macros run inside a "macro server" process built by the
//! compiler. Token streams are plain data structures here; the compiler
//! sends and receives them in a simple serialized form (see `__server`).

use std::fmt;
use std::iter::FromIterator;
use std::str::FromStr;

/// Whether proc_macro is usable (always true inside a macro server).
pub fn is_available() -> bool {
    true
}

// ---------------------------------------------------------------------------
// Span

/// A region of source code (spans are not tracked precisely).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Span {
    id: u32,
}

impl Span {
    pub fn call_site() -> Span { Span { id: 0 } }
    pub fn mixed_site() -> Span { Span { id: 0 } }
    pub fn def_site() -> Span { Span { id: 0 } }
    pub fn resolved_at(&self, _other: Span) -> Span { *self }
    pub fn located_at(&self, other: Span) -> Span { other }
    pub fn join(&self, _other: Span) -> Option<Span> { Some(*self) }
    pub fn source_text(&self) -> Option<String> { None }
    pub fn parent(&self) -> Option<Span> { None }
    pub fn source(&self) -> Span { *self }
    pub fn start(&self) -> LineColumn { LineColumn { line: 1, column: 0 } }
    pub fn end(&self) -> LineColumn { LineColumn { line: 1, column: 0 } }
    pub fn line(&self) -> usize { 1 }
    pub fn column(&self) -> usize { 1 }
    pub fn byte_range(&self) -> std::ops::Range<usize> { 0..0 }
    pub fn file(&self) -> String { String::new() }
    pub fn local_file(&self) -> Option<std::path::PathBuf> { None }
    pub fn unwrap(self) -> Span { self }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct LineColumn {
    pub line: usize,
    pub column: usize,
}

// ---------------------------------------------------------------------------
// Delimiter / Spacing

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Delimiter {
    Parenthesis,
    Brace,
    Bracket,
    None,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Spacing {
    Joint,
    Alone,
}

// ---------------------------------------------------------------------------
// TokenStream

/// An abstract stream of token trees.
#[derive(Clone, Default)]
pub struct TokenStream {
    trees: Vec<TokenTree>,
}

#[derive(Debug)]
pub struct LexError {
    msg: String,
}

impl fmt::Display for LexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.msg)
    }
}

impl std::error::Error for LexError {}

impl TokenStream {
    pub fn new() -> TokenStream {
        TokenStream { trees: Vec::new() }
    }
    pub fn is_empty(&self) -> bool {
        self.trees.is_empty()
    }
    pub fn expand_expr(&self) -> Result<TokenStream, ExpandError> {
        Err(ExpandError)
    }
}

#[derive(Debug)]
pub struct ExpandError;

impl FromStr for TokenStream {
    type Err = LexError;
    fn from_str(src: &str) -> Result<TokenStream, LexError> {
        __server::lex(src)
    }
}

impl From<TokenTree> for TokenStream {
    fn from(tree: TokenTree) -> TokenStream {
        TokenStream { trees: vec![tree] }
    }
}

impl FromIterator<TokenTree> for TokenStream {
    fn from_iter<I: IntoIterator<Item = TokenTree>>(iter: I) -> TokenStream {
        let mut trees = Vec::new();
        for t in iter {
            trees.push(t);
        }
        TokenStream { trees }
    }
}

impl FromIterator<TokenStream> for TokenStream {
    fn from_iter<I: IntoIterator<Item = TokenStream>>(iter: I) -> TokenStream {
        let mut trees = Vec::new();
        for s in iter {
            trees.extend(s.trees);
        }
        TokenStream { trees }
    }
}

impl Extend<TokenTree> for TokenStream {
    fn extend<I: IntoIterator<Item = TokenTree>>(&mut self, iter: I) {
        for t in iter {
            self.trees.push(t);
        }
    }
}

impl Extend<TokenStream> for TokenStream {
    fn extend<I: IntoIterator<Item = TokenStream>>(&mut self, iter: I) {
        for s in iter {
            self.trees.extend(s.trees);
        }
    }
}

pub mod token_stream {
    use super::{TokenStream, TokenTree};

    /// Iterator over the token trees of a stream.
    #[derive(Clone)]
    pub struct IntoIter {
        trees: std::vec::IntoIter<TokenTree>,
    }

    impl Iterator for IntoIter {
        type Item = TokenTree;
        fn next(&mut self) -> Option<TokenTree> {
            self.trees.next()
        }
        fn size_hint(&self) -> (usize, Option<usize>) {
            self.trees.size_hint()
        }
    }

    impl IntoIterator for TokenStream {
        type Item = TokenTree;
        type IntoIter = IntoIter;
        fn into_iter(self) -> IntoIter {
            IntoIter { trees: self.trees.into_iter() }
        }
    }
}

fn write_stream(f: &mut fmt::Formatter<'_>, trees: &[TokenTree]) -> fmt::Result {
    let mut joint = false;
    for (i, t) in trees.iter().enumerate() {
        if i > 0 && !joint {
            f.write_str(" ")?;
        }
        joint = false;
        match t {
            TokenTree::Group(g) => fmt::Display::fmt(g, f)?,
            TokenTree::Ident(id) => fmt::Display::fmt(id, f)?,
            TokenTree::Punct(p) => {
                fmt::Display::fmt(p, f)?;
                joint = p.spacing == Spacing::Joint;
            }
            TokenTree::Literal(l) => fmt::Display::fmt(l, f)?,
        }
    }
    Ok(())
}

impl fmt::Display for TokenStream {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_stream(f, &self.trees)
    }
}

impl fmt::Debug for TokenStream {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("TokenStream ")?;
        f.debug_list().entries(self.trees.iter()).finish()
    }
}

// ---------------------------------------------------------------------------
// TokenTree

#[derive(Clone)]
pub enum TokenTree {
    Group(Group),
    Ident(Ident),
    Punct(Punct),
    Literal(Literal),
}

impl TokenTree {
    pub fn span(&self) -> Span {
        match self {
            TokenTree::Group(t) => t.span(),
            TokenTree::Ident(t) => t.span(),
            TokenTree::Punct(t) => t.span(),
            TokenTree::Literal(t) => t.span(),
        }
    }
    pub fn set_span(&mut self, span: Span) {
        match self {
            TokenTree::Group(t) => t.set_span(span),
            TokenTree::Ident(t) => t.set_span(span),
            TokenTree::Punct(t) => t.set_span(span),
            TokenTree::Literal(t) => t.set_span(span),
        }
    }
}

impl From<Group> for TokenTree {
    fn from(g: Group) -> TokenTree { TokenTree::Group(g) }
}
impl From<Ident> for TokenTree {
    fn from(g: Ident) -> TokenTree { TokenTree::Ident(g) }
}
impl From<Punct> for TokenTree {
    fn from(g: Punct) -> TokenTree { TokenTree::Punct(g) }
}
impl From<Literal> for TokenTree {
    fn from(g: Literal) -> TokenTree { TokenTree::Literal(g) }
}

impl fmt::Display for TokenTree {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TokenTree::Group(t) => fmt::Display::fmt(t, f),
            TokenTree::Ident(t) => fmt::Display::fmt(t, f),
            TokenTree::Punct(t) => fmt::Display::fmt(t, f),
            TokenTree::Literal(t) => fmt::Display::fmt(t, f),
        }
    }
}

impl fmt::Debug for TokenTree {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TokenTree::Group(t) => fmt::Debug::fmt(t, f),
            TokenTree::Ident(t) => fmt::Debug::fmt(t, f),
            TokenTree::Punct(t) => fmt::Debug::fmt(t, f),
            TokenTree::Literal(t) => fmt::Debug::fmt(t, f),
        }
    }
}

// ---------------------------------------------------------------------------
// Group

#[derive(Clone)]
pub struct Group {
    delimiter: Delimiter,
    stream: TokenStream,
    span: Span,
}

impl Group {
    pub fn new(delimiter: Delimiter, stream: TokenStream) -> Group {
        Group { delimiter, stream, span: Span::call_site() }
    }
    pub fn delimiter(&self) -> Delimiter { self.delimiter }
    pub fn stream(&self) -> TokenStream { self.stream.clone() }
    pub fn span(&self) -> Span { self.span }
    pub fn span_open(&self) -> Span { self.span }
    pub fn span_close(&self) -> Span { self.span }
    pub fn set_span(&mut self, span: Span) { self.span = span; }
}

impl fmt::Display for Group {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (open, close) = match self.delimiter {
            Delimiter::Parenthesis => ("(", ")"),
            Delimiter::Brace => ("{ ", " }"),
            Delimiter::Bracket => ("[", "]"),
            Delimiter::None => ("", ""),
        };
        f.write_str(open)?;
        write_stream(f, &self.stream.trees)?;
        f.write_str(close)
    }
}

impl fmt::Debug for Group {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Group").field("delimiter", &self.delimiter).field("stream", &self.stream).finish()
    }
}

// ---------------------------------------------------------------------------
// Ident

#[derive(Clone)]
pub struct Ident {
    sym: String,
    raw: bool,
    span: Span,
}

impl Ident {
    pub fn new(string: &str, span: Span) -> Ident {
        if let Some(rest) = string.strip_prefix("r#") {
            return Ident { sym: rest.to_string(), raw: true, span };
        }
        Ident { sym: string.to_string(), raw: false, span }
    }
    pub fn new_raw(string: &str, span: Span) -> Ident {
        Ident { sym: string.to_string(), raw: true, span }
    }
    pub fn span(&self) -> Span { self.span }
    pub fn set_span(&mut self, span: Span) { self.span = span; }
}

impl fmt::Display for Ident {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.raw {
            f.write_str("r#")?;
        }
        f.write_str(&self.sym)
    }
}

impl fmt::Debug for Ident {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Ident").field("ident", &self.to_string()).finish()
    }
}

impl PartialEq for Ident {
    fn eq(&self, other: &Ident) -> bool {
        self.sym == other.sym && self.raw == other.raw
    }
}

// ---------------------------------------------------------------------------
// Punct

#[derive(Clone)]
pub struct Punct {
    ch: char,
    spacing: Spacing,
    span: Span,
}

impl Punct {
    pub fn new(ch: char, spacing: Spacing) -> Punct {
        Punct { ch, spacing, span: Span::call_site() }
    }
    pub fn as_char(&self) -> char { self.ch }
    pub fn spacing(&self) -> Spacing { self.spacing }
    pub fn span(&self) -> Span { self.span }
    pub fn set_span(&mut self, span: Span) { self.span = span; }
}

impl fmt::Display for Punct {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut buf = [0u8; 4];
        f.write_str(self.ch.encode_utf8(&mut buf))
    }
}

impl fmt::Debug for Punct {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Punct").field("ch", &self.ch).field("spacing", &self.spacing).finish()
    }
}

impl PartialEq<char> for Punct {
    fn eq(&self, other: &char) -> bool {
        self.ch == *other
    }
}

// ---------------------------------------------------------------------------
// Literal

#[derive(Clone)]
pub struct Literal {
    repr: String,
    span: Span,
}

macro_rules! suffixed_int_literals {
    ($($name:ident => $t:ty),*) => {
        $(
            pub fn $name(n: $t) -> Literal {
                Literal::from_repr(format!("{}{}", n, stringify!($t)))
            }
        )*
    };
}

macro_rules! unsuffixed_int_literals {
    ($($name:ident => $t:ty),*) => {
        $(
            pub fn $name(n: $t) -> Literal {
                Literal::from_repr(n.to_string())
            }
        )*
    };
}

impl Literal {
    fn from_repr(repr: String) -> Literal {
        Literal { repr, span: Span::call_site() }
    }

    suffixed_int_literals! {
        u8_suffixed => u8, u16_suffixed => u16, u32_suffixed => u32, u64_suffixed => u64,
        u128_suffixed => u128, usize_suffixed => usize, i8_suffixed => i8, i16_suffixed => i16,
        i32_suffixed => i32, i64_suffixed => i64, i128_suffixed => i128, isize_suffixed => isize
    }

    unsuffixed_int_literals! {
        u8_unsuffixed => u8, u16_unsuffixed => u16, u32_unsuffixed => u32, u64_unsuffixed => u64,
        u128_unsuffixed => u128, usize_unsuffixed => usize, i8_unsuffixed => i8, i16_unsuffixed => i16,
        i32_unsuffixed => i32, i64_unsuffixed => i64, i128_unsuffixed => i128, isize_unsuffixed => isize
    }

    pub fn f32_unsuffixed(n: f32) -> Literal {
        let mut s = n.to_string();
        if !s.contains('.') && !s.contains('e') && !s.contains("inf") && !s.contains("NaN") {
            s.push_str(".0");
        }
        Literal::from_repr(s)
    }
    pub fn f32_suffixed(n: f32) -> Literal {
        Literal::from_repr(format!("{}f32", n))
    }
    pub fn f64_unsuffixed(n: f64) -> Literal {
        let mut s = n.to_string();
        if !s.contains('.') && !s.contains('e') && !s.contains("inf") && !s.contains("NaN") {
            s.push_str(".0");
        }
        Literal::from_repr(s)
    }
    pub fn f64_suffixed(n: f64) -> Literal {
        Literal::from_repr(format!("{}f64", n))
    }
    pub fn string(s: &str) -> Literal {
        let mut r = String::from("\"");
        for c in s.chars() {
            match c {
                '"' => r.push_str("\\\""),
                '\\' => r.push_str("\\\\"),
                '\n' => r.push_str("\\n"),
                '\r' => r.push_str("\\r"),
                '\t' => r.push_str("\\t"),
                '\0' => r.push_str("\\0"),
                c => r.push(c),
            }
        }
        r.push('"');
        Literal::from_repr(r)
    }
    pub fn character(c: char) -> Literal {
        let r = match c {
            '\'' => String::from("'\\''"),
            '\\' => String::from("'\\\\'"),
            '\n' => String::from("'\\n'"),
            '\r' => String::from("'\\r'"),
            '\t' => String::from("'\\t'"),
            '\0' => String::from("'\\0'"),
            c => format!("'{}'", c),
        };
        Literal::from_repr(r)
    }
    pub fn byte_character(b: u8) -> Literal {
        Literal::from_repr(format!("b'\\x{:02x}'", b))
    }
    pub fn byte_string(bytes: &[u8]) -> Literal {
        let mut r = String::from("b\"");
        for &b in bytes {
            r.push_str(&format!("\\x{:02x}", b));
        }
        r.push('"');
        Literal::from_repr(r)
    }
    pub fn c_string(s: &std::ffi::CStr) -> Literal {
        let mut r = String::from("c\"");
        for &b in s.to_bytes() {
            r.push_str(&format!("\\x{:02x}", b));
        }
        r.push('"');
        Literal::from_repr(r)
    }
    pub fn span(&self) -> Span { self.span }
    pub fn set_span(&mut self, span: Span) { self.span = span; }
    pub fn subspan<R>(&self, _range: R) -> Option<Span> { None }
}

impl FromStr for Literal {
    type Err = LexError;
    fn from_str(src: &str) -> Result<Literal, LexError> {
        Ok(Literal::from_repr(src.trim().to_string()))
    }
}

impl fmt::Display for Literal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.repr)
    }
}

impl fmt::Debug for Literal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Literal").field("lit", &self.repr).finish()
    }
}

// ---------------------------------------------------------------------------
// Diagnostics (minimal)

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Level {
    Error,
    Warning,
    Note,
    Help,
}

pub struct Diagnostic {
    level: Level,
    message: String,
}

impl Diagnostic {
    pub fn new<T: Into<String>>(level: Level, message: T) -> Diagnostic {
        Diagnostic { level, message: message.into() }
    }
    pub fn spanned<S, T: Into<String>>(_spans: S, level: Level, message: T) -> Diagnostic {
        Diagnostic { level, message: message.into() }
    }
    pub fn emit(self) {
        if self.level == Level::Error {
            panic!("{}", self.message);
        }
    }
}

// ---------------------------------------------------------------------------
// The macro server (used by compiler-generated main functions).

#[doc(hidden)]
pub mod __server {
    use super::*;

    extern "C" {
        fn read(fd: i32, buf: *mut u8, n: usize) -> isize;
        fn write(fd: i32, buf: *const u8, n: usize) -> isize;
    }

    struct Io {
        buf: Vec<u8>,
        pos: usize,
    }

    static mut IO: Option<Io> = None;

    fn io() -> &'static mut Io {
        unsafe {
            let p = &raw mut IO;
            if (*p).is_none() {
                *p = Some(Io { buf: Vec::new(), pos: 0 });
            }
            (*p).as_mut().unwrap()
        }
    }

    fn fill(io: &mut Io) -> bool {
        let mut tmp = [0u8; 65536];
        let n = unsafe { read(0, tmp.as_mut_ptr(), tmp.len()) };
        if n <= 0 {
            return false;
        }
        if io.pos > 0 {
            io.buf.drain(..io.pos);
            io.pos = 0;
        }
        io.buf.extend_from_slice(&tmp[..n as usize]);
        true
    }

    fn read_line() -> Option<String> {
        let io = io();
        loop {
            if let Some(i) = io.buf[io.pos..].iter().position(|&b| b == b'\n') {
                let line = String::from_utf8_lossy(&io.buf[io.pos..io.pos + i]).to_string();
                io.pos += i + 1;
                return Some(line);
            }
            if !fill(io) {
                return None;
            }
        }
    }

    fn read_exact(n: usize) -> Option<Vec<u8>> {
        let io = io();
        while io.buf.len() - io.pos < n {
            if !fill(io) {
                return None;
            }
        }
        let out = io.buf[io.pos..io.pos + n].to_vec();
        io.pos += n;
        Some(out)
    }

    fn read_payload() -> Option<String> {
        let n: usize = read_line()?.trim().parse().ok()?;
        let bytes = read_exact(n)?;
        Some(String::from_utf8_lossy(&bytes).to_string())
    }

    fn write_all(s: &str) {
        let b = s.as_bytes();
        let mut off = 0;
        while off < b.len() {
            let n = unsafe { write(1, b.as_ptr().add(off), b.len() - off) };
            if n <= 0 {
                return;
            }
            off += n as usize;
        }
    }

    fn write_payload(s: &str) {
        write_all(&format!("{}\n", s.len()));
        write_all(s);
    }

    // ---- serialization ----

    fn put(out: &mut String, tag: char, s: &str) {
        out.push(tag);
        out.push_str(&s.len().to_string());
        out.push(':');
        out.push_str(s);
        out.push('\n');
    }

    pub fn serialize(ts: &TokenStream) -> String {
        let mut out = String::new();
        ser(&ts.trees, &mut out);
        out
    }

    fn ser(trees: &[TokenTree], out: &mut String) {
        for t in trees {
            match t {
                TokenTree::Group(g) => {
                    let tag = match g.delimiter {
                        Delimiter::Parenthesis => '(',
                        Delimiter::Bracket => '[',
                        Delimiter::Brace => '{',
                        Delimiter::None => 'N',
                    };
                    put(out, tag, "");
                    ser(&g.stream.trees, out);
                    put(out, ')', "");
                }
                TokenTree::Ident(id) => {
                    put(out, if id.raw { 'R' } else { 'I' }, &id.sym);
                }
                TokenTree::Punct(p) => {
                    let mut s = String::new();
                    s.push(p.ch);
                    s.push(if p.spacing == Spacing::Joint { 'J' } else { 'A' });
                    put(out, 'P', &s);
                }
                TokenTree::Literal(l) => put(out, 'L', &l.repr),
            }
        }
    }

    pub fn deserialize(s: &str) -> TokenStream {
        let bytes = s.as_bytes();
        let mut pos = 0usize;
        let mut stack: Vec<(Delimiter, Vec<TokenTree>)> = vec![(Delimiter::None, Vec::new())];
        while pos < bytes.len() {
            let tag = bytes[pos] as char;
            let mut j = pos + 1;
            while bytes[j] != b':' {
                j += 1;
            }
            let n: usize = s[pos + 1..j].parse().unwrap();
            let body = &s[j + 1..j + 1 + n];
            pos = j + 1 + n + 1;
            match tag {
                '(' => stack.push((Delimiter::Parenthesis, Vec::new())),
                '[' => stack.push((Delimiter::Bracket, Vec::new())),
                '{' => stack.push((Delimiter::Brace, Vec::new())),
                'N' => stack.push((Delimiter::None, Vec::new())),
                ')' => {
                    let (d, trees) = stack.pop().unwrap();
                    let g = Group { delimiter: d, stream: TokenStream { trees }, span: Span::call_site() };
                    stack.last_mut().unwrap().1.push(TokenTree::Group(g));
                }
                'I' | 'R' => {
                    let id = Ident { sym: body.to_string(), raw: tag == 'R', span: Span::call_site() };
                    stack.last_mut().unwrap().1.push(TokenTree::Ident(id));
                }
                'P' => {
                    let mut cs = body.chars();
                    let ch = cs.next().unwrap();
                    let sp = if cs.next() == Some('J') { Spacing::Joint } else { Spacing::Alone };
                    stack.last_mut().unwrap().1.push(TokenTree::Punct(Punct { ch, spacing: sp, span: Span::call_site() }));
                }
                _ => {
                    let l = Literal { repr: body.to_string(), span: Span::call_site() };
                    stack.last_mut().unwrap().1.push(TokenTree::Literal(l));
                }
            }
        }
        TokenStream { trees: stack.pop().unwrap().1 }
    }

    /// Asks the compiler to tokenize source text.
    pub fn lex(src: &str) -> Result<TokenStream, LexError> {
        write_all("LEX\n");
        write_payload(src);
        let status = read_line().unwrap_or_default();
        let payload = read_payload().unwrap_or_default();
        if status.trim() == "OK" {
            Ok(deserialize(&payload))
        } else {
            Err(LexError { msg: payload })
        }
    }

    /// A registered macro implementation.
    pub enum Macro {
        Derive(&'static str, fn(TokenStream) -> TokenStream),
        Attr(&'static str, fn(TokenStream, TokenStream) -> TokenStream),
        Bang(&'static str, fn(TokenStream) -> TokenStream),
    }

    /// Serves expansion requests until stdin is closed.
    pub fn run(macros: &[Macro]) {
        loop {
            let line = match read_line() {
                Some(l) => l,
                None => return,
            };
            let parts: Vec<&str> = line.split(' ').collect();
            if parts.len() != 3 || parts[0] != "REQ" {
                return;
            }
            let attr = deserialize(&read_payload().unwrap_or_default());
            let input = deserialize(&read_payload().unwrap_or_default());
            let mut result: Option<TokenStream> = None;
            for m in macros {
                match m {
                    Macro::Derive(name, f) if parts[1] == "derive" && *name == parts[2] => {
                        result = Some(f(input.clone()));
                    }
                    Macro::Attr(name, f) if parts[1] == "attr" && *name == parts[2] => {
                        result = Some(f(attr.clone(), input.clone()));
                    }
                    Macro::Bang(name, f) if parts[1] == "bang" && *name == parts[2] => {
                        result = Some(f(input.clone()));
                    }
                    _ => {}
                }
                if result.is_some() {
                    break;
                }
            }
            match result {
                Some(ts) => {
                    write_all("OK\n");
                    write_payload(&serialize(&ts));
                }
                None => {
                    write_all("PANIC\n");
                    write_payload(&format!("no proc macro `{}`", parts[2]));
                }
            }
        }
    }
}
