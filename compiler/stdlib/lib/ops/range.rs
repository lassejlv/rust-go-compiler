//! Range types.

use crate::fmt;
use crate::hash::{Hash, Hasher};

/// An unbounded range (`..`).
#[lang = "RangeFull"]
#[derive(Copy, Clone, Default, PartialEq, Eq, Hash)]
pub struct RangeFull;

impl fmt::Debug for RangeFull {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("..")
    }
}

/// A (half-open) range bounded inclusively below and exclusively above
/// (`start..end`).
#[lang = "Range"]
#[derive(Clone, Default, PartialEq, Eq, Hash)]
pub struct Range<Idx> {
    /// The lower bound of the range (inclusive).
    pub start: Idx,
    /// The upper bound of the range (exclusive).
    pub end: Idx,
}

impl<Idx: fmt::Debug> fmt::Debug for Range<Idx> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.start.fmt(f)?;
        f.write_str("..")?;
        self.end.fmt(f)
    }
}

impl<Idx: PartialOrd<Idx>> Range<Idx> {
    /// Returns `true` if `item` is contained in the range.
    pub fn contains<U>(&self, item: &U) -> bool
    where
        Idx: PartialOrd<U>,
        U: ?Sized + PartialOrd<Idx>,
    {
        self.start <= *item && *item < self.end
    }

    /// Returns `true` if the range contains no items.
    pub fn is_empty(&self) -> bool {
        !(self.start < self.end)
    }
}

/// A range only bounded inclusively below (`start..`).
#[lang = "RangeFrom"]
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct RangeFrom<Idx> {
    /// The lower bound of the range (inclusive).
    pub start: Idx,
}

impl<Idx: fmt::Debug> fmt::Debug for RangeFrom<Idx> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.start.fmt(f)?;
        f.write_str("..")
    }
}

impl<Idx: PartialOrd<Idx>> RangeFrom<Idx> {
    /// Returns `true` if `item` is contained in the range.
    pub fn contains<U>(&self, item: &U) -> bool
    where
        Idx: PartialOrd<U>,
        U: ?Sized + PartialOrd<Idx>,
    {
        self.start <= *item
    }
}

/// A range only bounded exclusively above (`..end`).
#[lang = "RangeTo"]
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct RangeTo<Idx> {
    /// The upper bound of the range (exclusive).
    pub end: Idx,
}

impl<Idx: fmt::Debug> fmt::Debug for RangeTo<Idx> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("..")?;
        self.end.fmt(f)
    }
}

impl<Idx: PartialOrd<Idx>> RangeTo<Idx> {
    /// Returns `true` if `item` is contained in the range.
    pub fn contains<U>(&self, item: &U) -> bool
    where
        Idx: PartialOrd<U>,
        U: ?Sized + PartialOrd<Idx>,
    {
        *item < self.end
    }
}

/// A range bounded inclusively below and above (`start..=end`).
#[lang = "RangeInclusive"]
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct RangeInclusive<Idx> {
    pub start: Idx,
    pub end: Idx,
    /// Set once iteration has yielded `end` (or the range was found empty).
    pub exhausted: bool,
}

impl<Idx> RangeInclusive<Idx> {
    /// Creates a new inclusive range. Equivalent to writing `start..=end`.
    #[inline]
    pub const fn new(start: Idx, end: Idx) -> Self {
        RangeInclusive { start, end, exhausted: false }
    }
    /// Returns the lower bound of the range (inclusive).
    #[inline]
    pub const fn start(&self) -> &Idx {
        &self.start
    }
    /// Returns the upper bound of the range (inclusive).
    #[inline]
    pub const fn end(&self) -> &Idx {
        &self.end
    }
    /// Destructures the `RangeInclusive` into (lower bound, upper (inclusive) bound).
    #[inline]
    pub fn into_inner(self) -> (Idx, Idx) {
        (self.start, self.end)
    }
}

impl<Idx: fmt::Debug> fmt::Debug for RangeInclusive<Idx> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.start.fmt(f)?;
        f.write_str("..=")?;
        self.end.fmt(f)?;
        if self.exhausted {
            f.write_str(" (exhausted)")?;
        }
        Ok(())
    }
}

impl<Idx: PartialOrd<Idx>> RangeInclusive<Idx> {
    /// Returns `true` if `item` is contained in the range.
    pub fn contains<U>(&self, item: &U) -> bool
    where
        Idx: PartialOrd<U>,
        U: ?Sized + PartialOrd<Idx>,
    {
        if self.exhausted {
            return false;
        }
        self.start <= *item && *item <= self.end
    }

    /// Returns `true` if the range contains no items.
    pub fn is_empty(&self) -> bool {
        self.exhausted || !(self.start <= self.end)
    }
}

/// A range only bounded inclusively above (`..=end`).
#[lang = "RangeToInclusive"]
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct RangeToInclusive<Idx> {
    /// The upper bound of the range (inclusive).
    pub end: Idx,
}

impl<Idx: fmt::Debug> fmt::Debug for RangeToInclusive<Idx> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("..=")?;
        self.end.fmt(f)
    }
}

impl<Idx: PartialOrd<Idx>> RangeToInclusive<Idx> {
    /// Returns `true` if `item` is contained in the range.
    pub fn contains<U>(&self, item: &U) -> bool
    where
        Idx: PartialOrd<U>,
        U: ?Sized + PartialOrd<Idx>,
    {
        *item <= self.end
    }
}

/// An endpoint of a range of keys.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Bound<T> {
    /// An inclusive bound.
    Included(T),
    /// An exclusive bound.
    Excluded(T),
    /// An infinite endpoint.
    Unbounded,
}

impl<T: Hash> Hash for Bound<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            Bound::Included(x) => {
                state.write_isize(0);
                x.hash(state)
            }
            Bound::Excluded(x) => {
                state.write_isize(1);
                x.hash(state)
            }
            Bound::Unbounded => state.write_isize(2),
        }
    }
}

impl<T> Bound<T> {
    /// Converts from `&Bound<T>` to `Bound<&T>`.
    pub fn as_ref(&self) -> Bound<&T> {
        match *self {
            Bound::Included(ref x) => Bound::Included(x),
            Bound::Excluded(ref x) => Bound::Excluded(x),
            Bound::Unbounded => Bound::Unbounded,
        }
    }
    /// Maps a `Bound<T>` to a `Bound<U>` by applying a function to the contained value.
    pub fn map<U, F: FnOnce(T) -> U>(self, f: F) -> Bound<U> {
        match self {
            Bound::Unbounded => Bound::Unbounded,
            Bound::Included(x) => Bound::Included(f(x)),
            Bound::Excluded(x) => Bound::Excluded(f(x)),
        }
    }
}

impl<T: Clone> Bound<&T> {
    /// Map a `Bound<&T>` to a `Bound<T>` by cloning the contents of the bound.
    pub fn cloned(self) -> Bound<T> {
        match self {
            Bound::Unbounded => Bound::Unbounded,
            Bound::Included(x) => Bound::Included(x.clone()),
            Bound::Excluded(x) => Bound::Excluded(x.clone()),
        }
    }
}

/// Implemented by Rust's built-in range types.
pub trait RangeBounds<T: ?Sized> {
    /// Start index bound.
    fn start_bound(&self) -> Bound<&T>;
    /// End index bound.
    fn end_bound(&self) -> Bound<&T>;

    /// Returns `true` if `item` is contained in the range.
    fn contains<U>(&self, item: &U) -> bool
    where
        T: PartialOrd<U>,
        U: ?Sized + PartialOrd<T>,
    {
        let start_ok = match self.start_bound() {
            Bound::Included(start) => *start <= *item,
            Bound::Excluded(start) => *start < *item,
            Bound::Unbounded => true,
        };
        if !start_ok {
            return false;
        }
        match self.end_bound() {
            Bound::Included(end) => *item <= *end,
            Bound::Excluded(end) => *item < *end,
            Bound::Unbounded => true,
        }
    }
}

impl<T: ?Sized> RangeBounds<T> for RangeFull {
    fn start_bound(&self) -> Bound<&T> {
        Bound::Unbounded
    }
    fn end_bound(&self) -> Bound<&T> {
        Bound::Unbounded
    }
}

impl<T> RangeBounds<T> for RangeFrom<T> {
    fn start_bound(&self) -> Bound<&T> {
        Bound::Included(&self.start)
    }
    fn end_bound(&self) -> Bound<&T> {
        Bound::Unbounded
    }
}

impl<T> RangeBounds<T> for RangeTo<T> {
    fn start_bound(&self) -> Bound<&T> {
        Bound::Unbounded
    }
    fn end_bound(&self) -> Bound<&T> {
        Bound::Excluded(&self.end)
    }
}

impl<T> RangeBounds<T> for Range<T> {
    fn start_bound(&self) -> Bound<&T> {
        Bound::Included(&self.start)
    }
    fn end_bound(&self) -> Bound<&T> {
        Bound::Excluded(&self.end)
    }
}

impl<T> RangeBounds<T> for RangeInclusive<T> {
    fn start_bound(&self) -> Bound<&T> {
        Bound::Included(&self.start)
    }
    fn end_bound(&self) -> Bound<&T> {
        if self.exhausted {
            Bound::Excluded(&self.end)
        } else {
            Bound::Included(&self.end)
        }
    }
}

impl<T> RangeBounds<T> for RangeToInclusive<T> {
    fn start_bound(&self) -> Bound<&T> {
        Bound::Unbounded
    }
    fn end_bound(&self) -> Bound<&T> {
        Bound::Included(&self.end)
    }
}

impl<T> RangeBounds<T> for (Bound<T>, Bound<T>) {
    fn start_bound(&self) -> Bound<&T> {
        self.0.as_ref()
    }
    fn end_bound(&self) -> Bound<&T> {
        self.1.as_ref()
    }
}

impl<'a, T: ?Sized + 'a> RangeBounds<T> for (Bound<&'a T>, Bound<&'a T>) {
    fn start_bound(&self) -> Bound<&T> {
        self.0
    }
    fn end_bound(&self) -> Bound<&T> {
        self.1
    }
}

impl<T> RangeBounds<T> for RangeFrom<&T> {
    fn start_bound(&self) -> Bound<&T> {
        Bound::Included(self.start)
    }
    fn end_bound(&self) -> Bound<&T> {
        Bound::Unbounded
    }
}

impl<T> RangeBounds<T> for RangeTo<&T> {
    fn start_bound(&self) -> Bound<&T> {
        Bound::Unbounded
    }
    fn end_bound(&self) -> Bound<&T> {
        Bound::Excluded(self.end)
    }
}

impl<T> RangeBounds<T> for Range<&T> {
    fn start_bound(&self) -> Bound<&T> {
        Bound::Included(self.start)
    }
    fn end_bound(&self) -> Bound<&T> {
        Bound::Excluded(self.end)
    }
}

impl<T> RangeBounds<T> for RangeInclusive<&T> {
    fn start_bound(&self) -> Bound<&T> {
        Bound::Included(self.start)
    }
    fn end_bound(&self) -> Bound<&T> {
        Bound::Included(self.end)
    }
}

impl<T> RangeBounds<T> for RangeToInclusive<&T> {
    fn start_bound(&self) -> Bound<&T> {
        Bound::Unbounded
    }
    fn end_bound(&self) -> Bound<&T> {
        Bound::Included(self.end)
    }
}
