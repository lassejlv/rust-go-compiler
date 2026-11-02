//! Utilities for formatting and printing strings.
//!
//! `format_args!` formats eagerly: `Arguments` wraps the already formatted
//! text. A `Formatter` owns its output buffer and the current format spec.

mod builders;
mod float;
mod impls;
mod num;

pub use self::builders::{DebugList, DebugMap, DebugSet, DebugStruct, DebugTuple};

use crate::marker::PhantomData;

/// The type returned by formatter methods.
pub type Result = crate::result::Result<(), Error>;

/// The error type which is returned from formatting a message into a stream.
#[derive(Copy, Clone, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Error;

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        Display::fmt("an error occurred when formatting an argument", f)
    }
}

impl crate::error::Error for Error {}

/// Possible alignments returned by `Formatter::align`.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Alignment {
    /// Indication that contents should be left-aligned.
    Left,
    /// Indication that contents should be right-aligned.
    Right,
    /// Indication that contents should be center-aligned.
    Center,
}

/// The signedness of a `Formatter` (`+` or `-` flag).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Sign {
    /// Represents the `+` flag.
    Plus,
    /// Represents the `-` flag.
    Minus,
}

/// A precompiled format string and its arguments (already formatted).
#[derive(Clone)]
pub struct Arguments<'a> {
    s: crate::string::String,
    _marker: PhantomData<&'a ()>,
}

impl<'a> Arguments<'a> {
    /// Creates `Arguments` from already formatted text (used by `format_args!`).
    #[inline]
    pub fn new(s: crate::string::String) -> Arguments<'a> {
        Arguments { s, _marker: PhantomData }
    }

    /// Gets the formatted string, if it has no arguments to be formatted at runtime.
    #[inline]
    pub fn as_str(&self) -> Option<&'static str> {
        None
    }

    /// The formatted text.
    #[inline]
    pub fn as_formatted_str(&self) -> &str {
        self.s.as_str()
    }

    #[inline]
    pub(crate) fn into_string(self) -> crate::string::String {
        self.s
    }
}

impl<'a> Debug for Arguments<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        f.write_str(self.s.as_str())
    }
}

impl<'a> Display for Arguments<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        f.write_str(self.s.as_str())
    }
}

/// Takes an `Arguments` struct and returns the resulting formatted string.
#[inline]
pub fn format(args: Arguments<'_>) -> crate::string::String {
    args.into_string()
}

/// Writes formatted output into a `Write` sink.
pub fn write<W: Write + ?Sized>(output: &mut W, args: Arguments<'_>) -> Result {
    output.write_str(args.as_formatted_str())
}

// Flag bits (as passed by `format_args!` to `set_spec`).
const FLAG_PLUS: u32 = 1;
const FLAG_MINUS: u32 = 2;
const FLAG_ALTERNATE: u32 = 4;
const FLAG_ZERO_PAD: u32 = 8;
const FLAG_DEBUG_LOWER_HEX: u32 = 16;
const FLAG_DEBUG_UPPER_HEX: u32 = 32;

// Alignment codes (as passed by `format_args!` to `set_spec`).
const ALIGN_UNKNOWN: u8 = 0;
const ALIGN_LEFT: u8 = 1;
const ALIGN_RIGHT: u8 = 2;
const ALIGN_CENTER: u8 = 3;

/// Configuration for formatting, plus the output buffer.
pub struct Formatter<'a> {
    buf: crate::string::String,
    fill: char,
    align: u8,
    flags: u32,
    width: Option<usize>,
    precision: Option<usize>,
    /// Number of `{:#?}` indentation levels currently active.
    depth: usize,
    /// Whether the last character written was a newline (for indentation).
    on_newline: bool,
    _marker: PhantomData<&'a ()>,
}

/// Padding written after the content.
pub(crate) struct PostPadding {
    fill: char,
    padding: usize,
}

impl PostPadding {
    fn write(self, f: &mut Formatter<'_>) -> Result {
        let mut i = 0;
        while i < self.padding {
            f.write_char(self.fill)?;
            i += 1;
        }
        Ok(())
    }
}

impl<'a> Formatter<'a> {
    /// Creates a formatter with an empty buffer and the default spec.
    #[inline]
    pub fn new() -> Formatter<'a> {
        Formatter {
            buf: crate::string::String::new(),
            fill: ' ',
            align: ALIGN_UNKNOWN,
            flags: 0,
            width: None,
            precision: None,
            depth: 0,
            on_newline: true,
            _marker: PhantomData,
        }
    }

    /// Sets the format spec used by the next `fmt` call (used by `format_args!`).
    #[inline]
    pub fn set_spec(&mut self, fill: char, align: u8, flags: u32, width: Option<usize>, precision: Option<usize>) {
        self.fill = fill;
        self.align = align;
        self.flags = flags;
        self.width = width;
        self.precision = precision;
    }

    /// Consumes the formatter, returning the formatted text.
    #[inline]
    pub fn into_string(self) -> crate::string::String {
        self.buf
    }

    /// Returns the text written so far.
    #[inline]
    pub fn as_str(&self) -> &str {
        self.buf.as_str()
    }

    /// Writes some data to the underlying buffer.
    #[inline]
    pub fn write_str(&mut self, data: &str) -> Result {
        if self.depth == 0 {
            self.buf.push_str(data);
            return Ok(());
        }
        let bytes = data.as_bytes();
        let n = bytes.len();
        let mut start = 0;
        while start < n {
            let mut end = start;
            while end < n && bytes[end] != b'\n' {
                end += 1;
            }
            let ends_with_nl = end < n;
            if ends_with_nl {
                end += 1;
            }
            if self.on_newline {
                let mut d = 0;
                while d < self.depth {
                    self.buf.push_str("    ");
                    d += 1;
                }
            }
            self.on_newline = ends_with_nl;
            self.buf.push_str(unsafe { data.get_unchecked(start..end) });
            start = end;
        }
        Ok(())
    }

    /// Writes a single character to the underlying buffer.
    #[inline]
    pub fn write_char(&mut self, c: char) -> Result {
        if self.depth == 0 || c != '\n' && !self.on_newline {
            self.buf.push(c);
            if self.depth != 0 {
                self.on_newline = false;
            }
            return Ok(());
        }
        let mut b = [0u8; 4];
        let s = c.encode_utf8(&mut b);
        self.write_str(&*s)
    }

    /// Writes some formatted information into this instance.
    #[inline]
    pub fn write_fmt(&mut self, fmt: Arguments<'_>) -> Result {
        self.write_str(fmt.as_formatted_str())
    }

    #[inline]
    pub(crate) fn push_indent(&mut self) {
        self.depth += 1;
    }

    #[inline]
    pub(crate) fn pop_indent(&mut self) {
        self.depth -= 1;
    }

    /// Writes `padding` fill characters before the content according to the
    /// alignment, returning the padding to write after the content.
    pub(crate) fn padding(&mut self, padding: usize, default: u8) -> crate::result::Result<PostPadding, Error> {
        let align = if self.align == ALIGN_UNKNOWN { default } else { self.align };
        let (pre_pad, post_pad) = match align {
            ALIGN_LEFT => (0, padding),
            ALIGN_CENTER => (padding / 2, (padding + 1) / 2),
            _ => (padding, 0),
        };
        let mut i = 0;
        while i < pre_pad {
            self.write_char(self.fill)?;
            i += 1;
        }
        Ok(PostPadding { fill: self.fill, padding: post_pad })
    }

    /// Performs the correct padding for an integer which has already been emitted into a str.
    pub fn pad_integral(&mut self, is_nonnegative: bool, prefix: &str, buf: &str) -> Result {
        let mut width = buf.len();
        let mut sign: Option<char> = None;
        if !is_nonnegative {
            sign = Some('-');
            width += 1;
        } else if self.sign_plus() {
            sign = Some('+');
            width += 1;
        }
        let prefix = if self.alternate() {
            width += prefix.chars().count();
            Some(prefix)
        } else {
            None
        };
        match self.width {
            None => {
                self.write_prefix(sign, prefix)?;
                self.write_str(buf)
            }
            Some(min) if width >= min => {
                self.write_prefix(sign, prefix)?;
                self.write_str(buf)
            }
            Some(min) if self.sign_aware_zero_pad() => {
                let old_fill = crate::mem::replace(&mut self.fill, '0');
                let old_align = crate::mem::replace(&mut self.align, ALIGN_RIGHT);
                self.write_prefix(sign, prefix)?;
                let post_padding = self.padding(min - width, ALIGN_RIGHT)?;
                self.write_str(buf)?;
                post_padding.write(self)?;
                self.fill = old_fill;
                self.align = old_align;
                Ok(())
            }
            Some(min) => {
                let post_padding = self.padding(min - width, ALIGN_RIGHT)?;
                self.write_prefix(sign, prefix)?;
                self.write_str(buf)?;
                post_padding.write(self)
            }
        }
    }

    fn write_prefix(&mut self, sign: Option<char>, prefix: Option<&str>) -> Result {
        if let Some(c) = sign {
            self.write_char(c)?;
        }
        if let Some(prefix) = prefix {
            self.write_str(prefix)
        } else {
            Ok(())
        }
    }

    /// Takes a string slice and emits it to the internal buffer after applying the relevant formatting flags specified.
    pub fn pad(&mut self, s: &str) -> Result {
        if self.width.is_none() && self.precision.is_none() {
            return self.write_str(s);
        }
        let s = if let Some(max) = self.precision {
            // Truncate to `max` characters.
            let mut it = s.char_indices();
            match it.nth(max) {
                Some((i, _)) => unsafe { s.get_unchecked(0..i) },
                None => s,
            }
        } else {
            s
        };
        match self.width {
            None => self.write_str(s),
            Some(width) => {
                let chars_count = s.chars().count();
                if chars_count >= width {
                    self.write_str(s)
                } else {
                    let post_padding = self.padding(width - chars_count, ALIGN_LEFT)?;
                    self.write_str(s)?;
                    post_padding.write(self)
                }
            }
        }
    }

    /// Writes a number that has already been rendered as `sign` + `body`
    /// (ASCII), applying width, fill, alignment and sign-aware zero padding.
    pub(crate) fn pad_formatted_parts(&mut self, sign: &str, body: &str) -> Result {
        match self.width {
            None => {
                self.write_str(sign)?;
                self.write_str(body)
            }
            Some(width) => {
                let mut width = width;
                let old_fill = self.fill;
                let old_align = self.align;
                let mut sign = sign;
                if self.sign_aware_zero_pad() {
                    self.write_str(sign)?;
                    width = width.saturating_sub(sign.len());
                    sign = "";
                    self.fill = '0';
                    self.align = ALIGN_RIGHT;
                }
                let len = sign.len() + body.len();
                let ret = if width <= len {
                    self.write_str(sign)?;
                    self.write_str(body)
                } else {
                    let post_padding = self.padding(width - len, ALIGN_RIGHT)?;
                    self.write_str(sign)?;
                    self.write_str(body)?;
                    post_padding.write(self)
                };
                self.fill = old_fill;
                self.align = old_align;
                ret
            }
        }
    }

    /// Character used as 'fill' whenever there is alignment.
    #[inline]
    pub fn fill(&self) -> char {
        self.fill
    }

    /// Flag indicating what form of alignment was requested.
    #[inline]
    pub fn align(&self) -> Option<Alignment> {
        match self.align {
            ALIGN_LEFT => Some(Alignment::Left),
            ALIGN_RIGHT => Some(Alignment::Right),
            ALIGN_CENTER => Some(Alignment::Center),
            _ => None,
        }
    }

    /// Optionally specified integer width that the output should be.
    #[inline]
    pub fn width(&self) -> Option<usize> {
        self.width
    }

    /// Optionally specified precision for numeric types.
    #[inline]
    pub fn precision(&self) -> Option<usize> {
        self.precision
    }

    /// Determines if the `+` flag was specified.
    #[inline]
    pub fn sign_plus(&self) -> bool {
        self.flags & FLAG_PLUS != 0
    }

    /// Determines if the `-` flag was specified.
    #[inline]
    pub fn sign_minus(&self) -> bool {
        self.flags & FLAG_MINUS != 0
    }

    /// Returns the sign flag, if any.
    #[inline]
    pub fn sign(&self) -> Option<Sign> {
        if self.sign_plus() {
            Some(Sign::Plus)
        } else if self.sign_minus() {
            Some(Sign::Minus)
        } else {
            None
        }
    }

    /// Determines if the `#` flag was specified.
    #[inline]
    pub fn alternate(&self) -> bool {
        self.flags & FLAG_ALTERNATE != 0
    }

    /// Determines if the `0` flag was specified.
    #[inline]
    pub fn sign_aware_zero_pad(&self) -> bool {
        self.flags & FLAG_ZERO_PAD != 0
    }

    /// Determines if the `x?` debug flag was specified.
    #[inline]
    pub fn debug_lower_hex(&self) -> bool {
        self.flags & FLAG_DEBUG_LOWER_HEX != 0
    }

    /// Determines if the `X?` debug flag was specified.
    #[inline]
    pub fn debug_upper_hex(&self) -> bool {
        self.flags & FLAG_DEBUG_UPPER_HEX != 0
    }

    /// Creates a `DebugStruct` builder designed to assist with creation of `Debug` implementations for structs.
    #[inline]
    pub fn debug_struct<'b>(&'b mut self, name: &str) -> DebugStruct<'b, 'a> {
        builders::debug_struct_new(self, name)
    }

    /// Creates a `DebugTuple` builder designed to assist with creation of `Debug` implementations for tuple structs.
    #[inline]
    pub fn debug_tuple<'b>(&'b mut self, name: &str) -> DebugTuple<'b, 'a> {
        builders::debug_tuple_new(self, name)
    }

    /// Creates a `DebugList` builder designed to assist with creation of `Debug` implementations for list-like structures.
    #[inline]
    pub fn debug_list<'b>(&'b mut self) -> DebugList<'b, 'a> {
        builders::debug_list_new(self)
    }

    /// Creates a `DebugSet` builder designed to assist with creation of `Debug` implementations for set-like structures.
    #[inline]
    pub fn debug_set<'b>(&'b mut self) -> DebugSet<'b, 'a> {
        builders::debug_set_new(self)
    }

    /// Creates a `DebugMap` builder designed to assist with creation of `Debug` implementations for map-like structures.
    #[inline]
    pub fn debug_map<'b>(&'b mut self) -> DebugMap<'b, 'a> {
        builders::debug_map_new(self)
    }
}

impl<'a> Write for Formatter<'a> {
    #[inline]
    fn write_str(&mut self, s: &str) -> Result {
        Formatter::write_str(self, s)
    }

    #[inline]
    fn write_char(&mut self, c: char) -> Result {
        Formatter::write_char(self, c)
    }

    #[inline]
    fn write_fmt(&mut self, args: Arguments<'_>) -> Result {
        Formatter::write_str(self, args.as_formatted_str())
    }
}

/// A trait for writing or formatting into Unicode-accepting buffers or streams.
pub trait Write {
    /// Writes a string slice into this writer, returning whether the write succeeded.
    fn write_str(&mut self, s: &str) -> Result;

    /// Writes a `char` into this writer, returning whether the write succeeded.
    fn write_char(&mut self, c: char) -> Result {
        let mut b = [0u8; 4];
        let s = c.encode_utf8(&mut b);
        self.write_str(&*s)
    }

    /// Glue for usage of the `write!` macro with implementors of this trait.
    fn write_fmt(&mut self, args: Arguments<'_>) -> Result {
        self.write_str(args.as_formatted_str())
    }
}

impl<W: Write + ?Sized> Write for &mut W {
    #[inline]
    fn write_str(&mut self, s: &str) -> Result {
        (**self).write_str(s)
    }

    #[inline]
    fn write_char(&mut self, c: char) -> Result {
        (**self).write_char(c)
    }

    #[inline]
    fn write_fmt(&mut self, args: Arguments<'_>) -> Result {
        (**self).write_fmt(args)
    }
}

/// `?` formatting.
pub trait Debug {
    /// Formats the value using the given formatter.
    fn fmt(&self, f: &mut Formatter<'_>) -> Result;
}

/// Format trait for an empty format, `{}`.
pub trait Display {
    /// Formats the value using the given formatter.
    fn fmt(&self, f: &mut Formatter<'_>) -> Result;
}

/// `o` formatting.
pub trait Octal {
    /// Formats the value using the given formatter.
    fn fmt(&self, f: &mut Formatter<'_>) -> Result;
}

/// `b` formatting.
pub trait Binary {
    /// Formats the value using the given formatter.
    fn fmt(&self, f: &mut Formatter<'_>) -> Result;
}

/// `x` formatting.
pub trait LowerHex {
    /// Formats the value using the given formatter.
    fn fmt(&self, f: &mut Formatter<'_>) -> Result;
}

/// `X` formatting.
pub trait UpperHex {
    /// Formats the value using the given formatter.
    fn fmt(&self, f: &mut Formatter<'_>) -> Result;
}

/// `p` formatting.
pub trait Pointer {
    /// Formats the value using the given formatter.
    fn fmt(&self, f: &mut Formatter<'_>) -> Result;
}

/// `e` formatting.
pub trait LowerExp {
    /// Formats the value using the given formatter.
    fn fmt(&self, f: &mut Formatter<'_>) -> Result;
}

/// `E` formatting.
pub trait UpperExp {
    /// Formats the value using the given formatter.
    fn fmt(&self, f: &mut Formatter<'_>) -> Result;
}

/// Formats a pointer address as `0x...` (the `{:p}` format).
pub(crate) fn pointer_fmt_inner(ptr_addr: usize, f: &mut Formatter<'_>) -> Result {
    let old_width = f.width;
    let old_flags = f.flags;
    if f.alternate() {
        f.flags |= FLAG_ZERO_PAD;
        if f.width.is_none() {
            f.width = Some((usize::BITS / 4) as usize + 2);
        }
    }
    f.flags |= FLAG_ALTERNATE;
    let ret = LowerHex::fmt(&ptr_addr, f);
    f.width = old_width;
    f.flags = old_flags;
    ret
}
