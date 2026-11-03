//! Helpers for implementing `Debug` (`debug_struct`, `debug_tuple`, ...).
//!
//! In alternate (`{:#?}`) mode every field is written with one more level of
//! indentation, which the `Formatter` applies after each newline.

use super::{Debug, Display, Formatter, Result};

/// A struct to help with `fmt::Debug` implementations (`Formatter::debug_struct`).
#[must_use]
pub struct DebugStruct<'a, 'b: 'a> {
    fmt: &'a mut Formatter<'b>,
    result: Result,
    has_fields: bool,
}

pub(super) fn debug_struct_new<'a, 'b>(fmt: &'a mut Formatter<'b>, name: &str) -> DebugStruct<'a, 'b> {
    let result = fmt.write_str(name);
    DebugStruct { fmt, result, has_fields: false }
}

impl<'a, 'b: 'a> DebugStruct<'a, 'b> {
    /// Adds a new field to the generated struct output.
    pub fn field(&mut self, name: &str, value: &dyn Debug) -> &mut Self {
        if self.result.is_ok() {
            self.result = if self.fmt.alternate() {
                pretty_struct_field(self.fmt, self.has_fields, name, value)
            } else {
                plain_struct_field(self.fmt, self.has_fields, name, value)
            };
        }
        self.has_fields = true;
        self
    }

    /// Marks the struct as non-exhaustive, indicating to the reader that there are some other fields that are not shown.
    pub fn finish_non_exhaustive(&mut self) -> Result {
        if self.result.is_ok() {
            self.result = if self.has_fields {
                if self.fmt.alternate() {
                    self.fmt.push_indent();
                    let r = self.fmt.write_str("..\n");
                    self.fmt.pop_indent();
                    match r {
                        Ok(()) => self.fmt.write_str("}"),
                        Err(e) => Err(e),
                    }
                } else {
                    self.fmt.write_str(", .. }")
                }
            } else {
                self.fmt.write_str(" { .. }")
            };
        }
        self.result
    }

    /// Finishes output and returns any error encountered.
    pub fn finish(&mut self) -> Result {
        if self.has_fields && self.result.is_ok() {
            self.result = if self.fmt.alternate() { self.fmt.write_str("}") } else { self.fmt.write_str(" }") };
        }
        self.result
    }
}

fn plain_struct_field(fmt: &mut Formatter<'_>, has_fields: bool, name: &str, value: &dyn Debug) -> Result {
    let prefix = if has_fields { ", " } else { " { " };
    fmt.write_str(prefix)?;
    fmt.write_str(name)?;
    fmt.write_str(": ")?;
    value.fmt(fmt)
}

fn pretty_struct_field(fmt: &mut Formatter<'_>, has_fields: bool, name: &str, value: &dyn Debug) -> Result {
    if !has_fields {
        fmt.write_str(" {\n")?;
    }
    fmt.push_indent();
    let r = pretty_struct_field_inner(fmt, name, value);
    fmt.pop_indent();
    r
}

fn pretty_struct_field_inner(fmt: &mut Formatter<'_>, name: &str, value: &dyn Debug) -> Result {
    fmt.write_str(name)?;
    fmt.write_str(": ")?;
    value.fmt(fmt)?;
    fmt.write_str(",\n")
}

/// A struct to help with `fmt::Debug` implementations (`Formatter::debug_tuple`).
#[must_use]
pub struct DebugTuple<'a, 'b: 'a> {
    fmt: &'a mut Formatter<'b>,
    result: Result,
    fields: usize,
    empty_name: bool,
}

pub(super) fn debug_tuple_new<'a, 'b>(fmt: &'a mut Formatter<'b>, name: &str) -> DebugTuple<'a, 'b> {
    let result = fmt.write_str(name);
    DebugTuple { fmt, result, fields: 0, empty_name: name.is_empty() }
}

impl<'a, 'b: 'a> DebugTuple<'a, 'b> {
    /// Adds a new field to the generated tuple struct output.
    pub fn field(&mut self, value: &dyn Debug) -> &mut Self {
        if self.result.is_ok() {
            self.result = if self.fmt.alternate() {
                pretty_tuple_field(self.fmt, self.fields == 0, value)
            } else {
                plain_tuple_field(self.fmt, self.fields == 0, value)
            };
        }
        self.fields += 1;
        self
    }

    /// Marks the tuple struct as non-exhaustive.
    pub fn finish_non_exhaustive(&mut self) -> Result {
        if self.result.is_ok() {
            self.result = if self.fields > 0 {
                if self.fmt.alternate() {
                    self.fmt.push_indent();
                    let r = self.fmt.write_str("..\n");
                    self.fmt.pop_indent();
                    match r {
                        Ok(()) => self.fmt.write_str(")"),
                        Err(e) => Err(e),
                    }
                } else {
                    self.fmt.write_str(", ..)")
                }
            } else {
                self.fmt.write_str("(..)")
            };
        }
        self.result
    }

    /// Finishes output and returns any error encountered.
    pub fn finish(&mut self) -> Result {
        if self.fields > 0 && self.result.is_ok() {
            if self.fields == 1 && self.empty_name && !self.fmt.alternate() {
                if let Err(e) = self.fmt.write_str(",") {
                    self.result = Err(e);
                    return self.result;
                }
            }
            self.result = self.fmt.write_str(")");
        }
        self.result
    }
}

fn plain_tuple_field(fmt: &mut Formatter<'_>, first: bool, value: &dyn Debug) -> Result {
    let prefix = if first { "(" } else { ", " };
    fmt.write_str(prefix)?;
    value.fmt(fmt)
}

fn pretty_tuple_field(fmt: &mut Formatter<'_>, first: bool, value: &dyn Debug) -> Result {
    if first {
        fmt.write_str("(\n")?;
    }
    fmt.push_indent();
    let r = pretty_entry_inner(fmt, value);
    fmt.pop_indent();
    r
}

fn pretty_entry_inner(fmt: &mut Formatter<'_>, value: &dyn Debug) -> Result {
    value.fmt(fmt)?;
    fmt.write_str(",\n")
}

// Shared implementation of lists and sets.
struct DebugInner<'a, 'b: 'a> {
    fmt: &'a mut Formatter<'b>,
    result: Result,
    has_fields: bool,
}

impl<'a, 'b: 'a> DebugInner<'a, 'b> {
    fn entry_dyn(&mut self, value: &dyn Debug) {
        if self.result.is_ok() {
            self.result = if self.fmt.alternate() {
                pretty_inner_entry(self.fmt, self.has_fields, value)
            } else {
                plain_inner_entry(self.fmt, self.has_fields, value)
            };
        }
        self.has_fields = true;
    }

    fn is_pretty(&self) -> bool {
        self.fmt.alternate()
    }
}

fn plain_inner_entry(fmt: &mut Formatter<'_>, has_fields: bool, value: &dyn Debug) -> Result {
    if has_fields {
        fmt.write_str(", ")?;
    }
    value.fmt(fmt)
}

fn pretty_inner_entry(fmt: &mut Formatter<'_>, has_fields: bool, value: &dyn Debug) -> Result {
    if !has_fields {
        fmt.write_str("\n")?;
    }
    fmt.push_indent();
    let r = pretty_entry_inner(fmt, value);
    fmt.pop_indent();
    r
}

/// A struct to help with `fmt::Debug` implementations (`Formatter::debug_set`).
#[must_use]
pub struct DebugSet<'a, 'b: 'a> {
    inner: DebugInner<'a, 'b>,
}

pub(super) fn debug_set_new<'a, 'b>(fmt: &'a mut Formatter<'b>) -> DebugSet<'a, 'b> {
    let result = fmt.write_str("{");
    DebugSet { inner: DebugInner { fmt, result, has_fields: false } }
}

impl<'a, 'b: 'a> DebugSet<'a, 'b> {
    /// Adds a new entry to the set output.
    pub fn entry(&mut self, entry: &dyn Debug) -> &mut Self {
        self.inner.entry_dyn(entry);
        self
    }

    /// Adds the contents of an iterator of entries to the set output.
    pub fn entries<D, I>(&mut self, entries: I) -> &mut Self
    where
        D: Debug,
        I: IntoIterator<Item = D>,
    {
        let mut it = entries.into_iter();
        while let Some(entry) = it.next() {
            self.inner.entry_dyn(&entry);
        }
        self
    }

    /// Marks the set as non-exhaustive.
    pub fn finish_non_exhaustive(&mut self) -> Result {
        finish_inner_non_exhaustive(&mut self.inner, "}")
    }

    /// Finishes output and returns any error encountered.
    pub fn finish(&mut self) -> Result {
        if self.inner.result.is_ok() {
            self.inner.result = self.inner.fmt.write_str("}");
        }
        self.inner.result
    }
}

fn finish_inner_non_exhaustive(inner: &mut DebugInner<'_, '_>, close: &str) -> Result {
    if inner.result.is_ok() {
        inner.result = if inner.has_fields {
            if inner.is_pretty() {
                inner.fmt.push_indent();
                let r = inner.fmt.write_str("..\n");
                inner.fmt.pop_indent();
                match r {
                    Ok(()) => inner.fmt.write_str(close),
                    Err(e) => Err(e),
                }
            } else {
                match inner.fmt.write_str(", ..") {
                    Ok(()) => inner.fmt.write_str(close),
                    Err(e) => Err(e),
                }
            }
        } else {
            match inner.fmt.write_str("..") {
                Ok(()) => inner.fmt.write_str(close),
                Err(e) => Err(e),
            }
        };
    }
    inner.result
}

/// A struct to help with `fmt::Debug` implementations (`Formatter::debug_list`).
#[must_use]
pub struct DebugList<'a, 'b: 'a> {
    inner: DebugInner<'a, 'b>,
}

pub(super) fn debug_list_new<'a, 'b>(fmt: &'a mut Formatter<'b>) -> DebugList<'a, 'b> {
    let result = fmt.write_str("[");
    DebugList { inner: DebugInner { fmt, result, has_fields: false } }
}

impl<'a, 'b: 'a> DebugList<'a, 'b> {
    /// Adds a new entry to the list output.
    pub fn entry(&mut self, entry: &dyn Debug) -> &mut Self {
        self.inner.entry_dyn(entry);
        self
    }

    /// Adds the contents of an iterator of entries to the list output.
    pub fn entries<D, I>(&mut self, entries: I) -> &mut Self
    where
        D: Debug,
        I: IntoIterator<Item = D>,
    {
        let mut it = entries.into_iter();
        while let Some(entry) = it.next() {
            self.inner.entry_dyn(&entry);
        }
        self
    }

    /// Marks the list as non-exhaustive.
    pub fn finish_non_exhaustive(&mut self) -> Result {
        finish_inner_non_exhaustive(&mut self.inner, "]")
    }

    /// Finishes output and returns any error encountered.
    pub fn finish(&mut self) -> Result {
        if self.inner.result.is_ok() {
            self.inner.result = self.inner.fmt.write_str("]");
        }
        self.inner.result
    }
}

/// A struct to help with `fmt::Debug` implementations (`Formatter::debug_map`).
#[must_use]
pub struct DebugMap<'a, 'b: 'a> {
    fmt: &'a mut Formatter<'b>,
    result: Result,
    has_fields: bool,
    has_key: bool,
}

pub(super) fn debug_map_new<'a, 'b>(fmt: &'a mut Formatter<'b>) -> DebugMap<'a, 'b> {
    let result = fmt.write_str("{");
    DebugMap { fmt, result, has_fields: false, has_key: false }
}

impl<'a, 'b: 'a> DebugMap<'a, 'b> {
    /// Adds a new entry to the map output.
    pub fn entry(&mut self, key: &dyn Debug, value: &dyn Debug) -> &mut Self {
        self.key(key).value(value)
    }

    /// Adds the key part of a new entry to the map output.
    pub fn key(&mut self, key: &dyn Debug) -> &mut Self {
        if self.result.is_ok() {
            if self.has_key {
                crate::rt::panic_str(
                    "attempted to begin a new map entry without completing the previous one",
                    file!(),
                    line!(),
                    column!(),
                );
            }
            self.result = if self.fmt.alternate() {
                pretty_map_key(self.fmt, self.has_fields, key)
            } else {
                plain_map_key(self.fmt, self.has_fields, key)
            };
            self.has_key = true;
        }
        self
    }

    /// Adds the value part of a new entry to the map output.
    pub fn value(&mut self, value: &dyn Debug) -> &mut Self {
        if self.result.is_ok() {
            if !self.has_key {
                crate::rt::panic_str("attempted to format a map value before its key", file!(), line!(), column!());
            }
            self.result = if self.fmt.alternate() {
                self.fmt.push_indent();
                let r = pretty_entry_inner(self.fmt, value);
                self.fmt.pop_indent();
                r
            } else {
                value.fmt(self.fmt)
            };
            self.has_key = false;
        }
        self.has_fields = true;
        self
    }

    /// Adds the contents of an iterator of entries to the map output.
    pub fn entries<K, V, I>(&mut self, entries: I) -> &mut Self
    where
        K: Debug,
        V: Debug,
        I: IntoIterator<Item = (K, V)>,
    {
        let mut it = entries.into_iter();
        while let Some((k, v)) = it.next() {
            self.entry(&k, &v);
        }
        self
    }

    /// Marks the map as non-exhaustive.
    pub fn finish_non_exhaustive(&mut self) -> Result {
        if self.result.is_ok() {
            self.result = if self.has_fields {
                if self.fmt.alternate() {
                    self.fmt.push_indent();
                    let r = self.fmt.write_str("..\n");
                    self.fmt.pop_indent();
                    match r {
                        Ok(()) => self.fmt.write_str("}"),
                        Err(e) => Err(e),
                    }
                } else {
                    self.fmt.write_str(", ..}")
                }
            } else {
                self.fmt.write_str("..}")
            };
        }
        self.result
    }

    /// Finishes output and returns any error encountered.
    pub fn finish(&mut self) -> Result {
        if self.result.is_ok() {
            if self.has_key {
                crate::rt::panic_str("attempted to finish a map with a partial entry", file!(), line!(), column!());
            }
            self.result = self.fmt.write_str("}");
        }
        self.result
    }
}

fn plain_map_key(fmt: &mut Formatter<'_>, has_fields: bool, key: &dyn Debug) -> Result {
    if has_fields {
        fmt.write_str(", ")?;
    }
    key.fmt(fmt)?;
    fmt.write_str(": ")
}

fn pretty_map_key(fmt: &mut Formatter<'_>, has_fields: bool, key: &dyn Debug) -> Result {
    if !has_fields {
        fmt.write_str("\n")?;
    }
    fmt.push_indent();
    let r = pretty_map_key_inner(fmt, key);
    fmt.pop_indent();
    r
}

fn pretty_map_key_inner(fmt: &mut Formatter<'_>, key: &dyn Debug) -> Result {
    key.fmt(fmt)?;
    fmt.write_str(": ")
}

/// Creates a type whose `Debug` and `Display` impls are provided with the function `f`.
pub fn from_fn<F: Fn(&mut Formatter<'_>) -> Result>(f: F) -> FromFn<F> {
    FromFn(f)
}

/// Implements `Debug` and `Display` using a function.
pub struct FromFn<F>(F)
where
    F: Fn(&mut Formatter<'_>) -> Result;

impl<F> Debug for FromFn<F>
where
    F: Fn(&mut Formatter<'_>) -> Result,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        (self.0)(f)
    }
}

impl<F> Display for FromFn<F>
where
    F: Fn(&mut Formatter<'_>) -> Result,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        (self.0)(f)
    }
}
