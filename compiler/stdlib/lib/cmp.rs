//! Utilities for comparing and ordering values.

/// Trait for comparisons using the equality operator (`==`, `!=`).
#[lang = "eq"]
pub trait PartialEq<Rhs: ?Sized = Self> {
    /// Tests for `self` and `other` values to be equal.
    fn eq(&self, other: &Rhs) -> bool;

    /// Tests for `!=`.
    #[inline]
    fn ne(&self, other: &Rhs) -> bool {
        !self.eq(other)
    }
}

/// Trait for equality comparisons which are equivalence relations.
pub trait Eq: PartialEq<Self> {
    #[doc(hidden)]
    fn assert_receiver_is_total_eq(&self) {}
}

/// The result of a comparison between two values.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
#[repr(i8)]
pub enum Ordering {
    /// An ordering where a compared value is less than another.
    Less = -1,
    /// An ordering where a compared value is equal to another.
    Equal = 0,
    /// An ordering where a compared value is greater than another.
    Greater = 1,
}

impl Ordering {
    #[inline]
    pub const fn is_eq(self) -> bool {
        match self {
            Ordering::Equal => true,
            _ => false,
        }
    }
    #[inline]
    pub const fn is_ne(self) -> bool {
        !self.is_eq()
    }
    #[inline]
    pub const fn is_lt(self) -> bool {
        match self {
            Ordering::Less => true,
            _ => false,
        }
    }
    #[inline]
    pub const fn is_gt(self) -> bool {
        match self {
            Ordering::Greater => true,
            _ => false,
        }
    }
    #[inline]
    pub const fn is_le(self) -> bool {
        !self.is_gt()
    }
    #[inline]
    pub const fn is_ge(self) -> bool {
        !self.is_lt()
    }
    /// Reverses the `Ordering`.
    #[inline]
    pub const fn reverse(self) -> Ordering {
        match self {
            Ordering::Less => Ordering::Greater,
            Ordering::Equal => Ordering::Equal,
            Ordering::Greater => Ordering::Less,
        }
    }
    /// Chains two orderings.
    #[inline]
    pub const fn then(self, other: Ordering) -> Ordering {
        match self {
            Ordering::Equal => other,
            _ => self,
        }
    }
    /// Chains the ordering with the given function.
    #[inline]
    pub fn then_with<F: FnOnce() -> Ordering>(self, f: F) -> Ordering {
        match self {
            Ordering::Equal => f(),
            _ => self,
        }
    }
    #[inline]
    fn as_i8(self) -> i8 {
        match self {
            Ordering::Less => -1,
            Ordering::Equal => 0,
            Ordering::Greater => 1,
        }
    }
}

impl PartialOrd for Ordering {
    #[inline]
    fn partial_cmp(&self, other: &Ordering) -> Option<Ordering> {
        self.as_i8().partial_cmp(&other.as_i8())
    }
}

impl Ord for Ordering {
    #[inline]
    fn cmp(&self, other: &Ordering) -> Ordering {
        self.as_i8().cmp(&other.as_i8())
    }
}

impl Default for Ordering {
    fn default() -> Ordering {
        Ordering::Equal
    }
}

/// A helper struct for reverse ordering.
#[derive(PartialEq, Eq, Debug, Copy, Clone, Default, Hash)]
pub struct Reverse<T>(pub T);

impl<T: PartialOrd> PartialOrd for Reverse<T> {
    #[inline]
    fn partial_cmp(&self, other: &Reverse<T>) -> Option<Ordering> {
        other.0.partial_cmp(&self.0)
    }
    #[inline]
    fn lt(&self, other: &Self) -> bool {
        other.0 < self.0
    }
    #[inline]
    fn le(&self, other: &Self) -> bool {
        other.0 <= self.0
    }
    #[inline]
    fn gt(&self, other: &Self) -> bool {
        other.0 > self.0
    }
    #[inline]
    fn ge(&self, other: &Self) -> bool {
        other.0 >= self.0
    }
}

impl<T: Ord> Ord for Reverse<T> {
    #[inline]
    fn cmp(&self, other: &Reverse<T>) -> Ordering {
        other.0.cmp(&self.0)
    }
}

/// Trait for types that form a total order.
pub trait Ord: Eq + PartialOrd<Self> {
    /// Returns an `Ordering` between `self` and `other`.
    fn cmp(&self, other: &Self) -> Ordering;

    /// Compares and returns the maximum of two values (the second one if equal).
    #[inline]
    fn max(self, other: Self) -> Self
    where
        Self: Sized,
    {
        if other < self {
            self
        } else {
            other
        }
    }

    /// Compares and returns the minimum of two values (the first one if equal).
    #[inline]
    fn min(self, other: Self) -> Self
    where
        Self: Sized,
    {
        if other < self {
            other
        } else {
            self
        }
    }

    /// Restricts a value to a certain interval.
    fn clamp(self, min: Self, max: Self) -> Self
    where
        Self: Sized,
    {
        if min > max {
            crate::rt::panic_str("assertion failed: min <= max", file!(), line!(), column!());
        }
        if self < min {
            min
        } else if self > max {
            max
        } else {
            self
        }
    }
}

/// Trait for types that form a partial order.
#[lang = "partial_ord"]
pub trait PartialOrd<Rhs: ?Sized = Self>: PartialEq<Rhs> {
    /// Returns an ordering between `self` and `other` values if one exists.
    fn partial_cmp(&self, other: &Rhs) -> Option<Ordering>;

    /// Tests less than (for `self` and `other`) and is used by the `<` operator.
    #[inline]
    fn lt(&self, other: &Rhs) -> bool {
        match self.partial_cmp(other) {
            Some(Ordering::Less) => true,
            _ => false,
        }
    }

    /// Tests less than or equal to (`<=`).
    #[inline]
    fn le(&self, other: &Rhs) -> bool {
        match self.partial_cmp(other) {
            Some(Ordering::Less) | Some(Ordering::Equal) => true,
            _ => false,
        }
    }

    /// Tests greater than (`>`).
    #[inline]
    fn gt(&self, other: &Rhs) -> bool {
        match self.partial_cmp(other) {
            Some(Ordering::Greater) => true,
            _ => false,
        }
    }

    /// Tests greater than or equal to (`>=`).
    #[inline]
    fn ge(&self, other: &Rhs) -> bool {
        match self.partial_cmp(other) {
            Some(Ordering::Greater) | Some(Ordering::Equal) => true,
            _ => false,
        }
    }
}

/// Compares and returns the minimum of two values.
#[inline]
pub fn min<T: Ord>(v1: T, v2: T) -> T {
    v1.min(v2)
}

/// Returns the minimum of two values with respect to the specified comparison function.
#[inline]
pub fn min_by<T, F: FnOnce(&T, &T) -> Ordering>(v1: T, v2: T, compare: F) -> T {
    match compare(&v1, &v2) {
        Ordering::Less | Ordering::Equal => v1,
        Ordering::Greater => v2,
    }
}

/// Returns the element that gives the minimum value from the specified function.
#[inline]
pub fn min_by_key<T, F: FnMut(&T) -> K, K: Ord>(v1: T, v2: T, mut f: F) -> T {
    let k1 = f(&v1);
    let k2 = f(&v2);
    if k2 < k1 {
        v2
    } else {
        v1
    }
}

/// Compares and returns the maximum of two values.
#[inline]
pub fn max<T: Ord>(v1: T, v2: T) -> T {
    v1.max(v2)
}

/// Returns the maximum of two values with respect to the specified comparison function.
#[inline]
pub fn max_by<T, F: FnOnce(&T, &T) -> Ordering>(v1: T, v2: T, compare: F) -> T {
    match compare(&v1, &v2) {
        Ordering::Less | Ordering::Equal => v2,
        Ordering::Greater => v1,
    }
}

/// Returns the element that gives the maximum value from the specified function.
#[inline]
pub fn max_by_key<T, F: FnMut(&T) -> K, K: Ord>(v1: T, v2: T, mut f: F) -> T {
    let k1 = f(&v1);
    let k2 = f(&v2);
    if k2 < k1 {
        v1
    } else {
        v2
    }
}

// ---------------------------------------------------------------------------
// Implementations for primitive types

macro_rules! __std_cmp_partial_eq_prim {
    ($($t:ty)*) => {
        $(
            impl PartialEq for $t {
                #[inline]
                fn eq(&self, other: &$t) -> bool {
                    (*self) == (*other)
                }
                #[inline]
                fn ne(&self, other: &$t) -> bool {
                    (*self) != (*other)
                }
            }
        )*
    };
}

__std_cmp_partial_eq_prim! { bool char i8 i16 i32 i64 i128 isize u8 u16 u32 u64 u128 usize f32 f64 }

macro_rules! __std_cmp_eq_prim {
    ($($t:ty)*) => {
        $(
            impl Eq for $t {}
        )*
    };
}

__std_cmp_eq_prim! { bool char i8 i16 i32 i64 i128 isize u8 u16 u32 u64 u128 usize }

macro_rules! __std_cmp_ord_prim {
    ($($t:ty)*) => {
        $(
            impl PartialOrd for $t {
                #[inline]
                fn partial_cmp(&self, other: &$t) -> Option<Ordering> {
                    Some(Ord::cmp(self, other))
                }
                #[inline]
                fn lt(&self, other: &$t) -> bool { (*self) < (*other) }
                #[inline]
                fn le(&self, other: &$t) -> bool { (*self) <= (*other) }
                #[inline]
                fn gt(&self, other: &$t) -> bool { (*self) > (*other) }
                #[inline]
                fn ge(&self, other: &$t) -> bool { (*self) >= (*other) }
            }

            impl Ord for $t {
                #[inline]
                fn cmp(&self, other: &$t) -> Ordering {
                    if (*self) < (*other) {
                        Ordering::Less
                    } else if (*self) == (*other) {
                        Ordering::Equal
                    } else {
                        Ordering::Greater
                    }
                }

                #[inline]
                #[track_caller]
                fn clamp(self, min: $t, max: $t) -> $t {
                    if min > max {
                        panic!("min > max. min = {:?}, max = {:?}", min, max);
                    }
                    if self < min {
                        min
                    } else if self > max {
                        max
                    } else {
                        self
                    }
                }
            }
        )*
    };
}

__std_cmp_ord_prim! { char i8 i16 i32 i64 i128 isize u8 u16 u32 u64 u128 usize }

impl PartialOrd for bool {
    #[inline]
    fn partial_cmp(&self, other: &bool) -> Option<Ordering> {
        Some(Ord::cmp(self, other))
    }
}

impl Ord for bool {
    #[inline]
    fn cmp(&self, other: &bool) -> Ordering {
        let a = (*self) as u8;
        let b = (*other) as u8;
        a.cmp(&b)
    }
}

macro_rules! __std_cmp_partial_ord_float {
    ($($t:ty)*) => {
        $(
            impl PartialOrd for $t {
                #[inline]
                fn partial_cmp(&self, other: &$t) -> Option<Ordering> {
                    let a = *self;
                    let b = *other;
                    if a < b {
                        Some(Ordering::Less)
                    } else if a > b {
                        Some(Ordering::Greater)
                    } else if a == b {
                        Some(Ordering::Equal)
                    } else {
                        None
                    }
                }
                #[inline]
                fn lt(&self, other: &$t) -> bool { (*self) < (*other) }
                #[inline]
                fn le(&self, other: &$t) -> bool { (*self) <= (*other) }
                #[inline]
                fn gt(&self, other: &$t) -> bool { (*self) > (*other) }
                #[inline]
                fn ge(&self, other: &$t) -> bool { (*self) >= (*other) }
            }
        )*
    };
}

__std_cmp_partial_ord_float! { f32 f64 }

impl PartialEq for () {
    #[inline]
    fn eq(&self, _other: &()) -> bool {
        true
    }
}

impl Eq for () {}

impl PartialOrd for () {
    #[inline]
    fn partial_cmp(&self, _other: &()) -> Option<Ordering> {
        Some(Ordering::Equal)
    }
}

impl Ord for () {
    #[inline]
    fn cmp(&self, _other: &()) -> Ordering {
        Ordering::Equal
    }
}

// ---------------------------------------------------------------------------
// References

impl<'a, 'b, A: ?Sized, B: ?Sized> PartialEq<&'b B> for &'a A
where
    A: PartialEq<B>,
{
    #[inline]
    fn eq(&self, other: &&'b B) -> bool {
        PartialEq::eq(*self, *other)
    }
    #[inline]
    fn ne(&self, other: &&'b B) -> bool {
        PartialEq::ne(*self, *other)
    }
}

impl<'a, 'b, A: ?Sized, B: ?Sized> PartialEq<&'b mut B> for &'a mut A
where
    A: PartialEq<B>,
{
    #[inline]
    fn eq(&self, other: &&'b mut B) -> bool {
        PartialEq::eq(*self, *other)
    }
}

impl<'a, 'b, A: ?Sized, B: ?Sized> PartialEq<&'b mut B> for &'a A
where
    A: PartialEq<B>,
{
    #[inline]
    fn eq(&self, other: &&'b mut B) -> bool {
        PartialEq::eq(*self, *other)
    }
}

impl<'a, 'b, A: ?Sized, B: ?Sized> PartialEq<&'b B> for &'a mut A
where
    A: PartialEq<B>,
{
    #[inline]
    fn eq(&self, other: &&'b B) -> bool {
        PartialEq::eq(*self, *other)
    }
}

impl<A: ?Sized + Eq> Eq for &A {}
impl<A: ?Sized + Eq> Eq for &mut A {}

impl<'a, 'b, A: ?Sized, B: ?Sized> PartialOrd<&'b B> for &'a A
where
    A: PartialOrd<B>,
{
    #[inline]
    fn partial_cmp(&self, other: &&'b B) -> Option<Ordering> {
        PartialOrd::partial_cmp(*self, *other)
    }
    #[inline]
    fn lt(&self, other: &&'b B) -> bool {
        PartialOrd::lt(*self, *other)
    }
    #[inline]
    fn le(&self, other: &&'b B) -> bool {
        PartialOrd::le(*self, *other)
    }
    #[inline]
    fn gt(&self, other: &&'b B) -> bool {
        PartialOrd::gt(*self, *other)
    }
    #[inline]
    fn ge(&self, other: &&'b B) -> bool {
        PartialOrd::ge(*self, *other)
    }
}

impl<'a, 'b, A: ?Sized, B: ?Sized> PartialOrd<&'b mut B> for &'a mut A
where
    A: PartialOrd<B>,
{
    #[inline]
    fn partial_cmp(&self, other: &&'b mut B) -> Option<Ordering> {
        PartialOrd::partial_cmp(*self, *other)
    }
}

impl<A: ?Sized + Ord> Ord for &A {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        Ord::cmp(*self, *other)
    }
}

impl<A: ?Sized + Ord> Ord for &mut A {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        Ord::cmp(*self, *other)
    }
}

// ---------------------------------------------------------------------------
// Raw pointers (address comparison)

impl<T: ?Sized> PartialEq for *const T {
    #[inline]
    fn eq(&self, other: &*const T) -> bool {
        (*self as *const u8 as usize) == (*other as *const u8 as usize)
    }
}

impl<T: ?Sized> Eq for *const T {}

impl<T: ?Sized> PartialEq for *mut T {
    #[inline]
    fn eq(&self, other: &*mut T) -> bool {
        (*self as *const u8 as usize) == (*other as *const u8 as usize)
    }
}

impl<T: ?Sized> Eq for *mut T {}

impl<T: ?Sized> PartialOrd for *const T {
    fn partial_cmp(&self, other: &*const T) -> Option<Ordering> {
        (*self as *const u8 as usize).partial_cmp(&(*other as *const u8 as usize))
    }
}

impl<T: ?Sized> Ord for *const T {
    fn cmp(&self, other: &*const T) -> Ordering {
        (*self as *const u8 as usize).cmp(&(*other as *const u8 as usize))
    }
}

impl<T: ?Sized> PartialOrd for *mut T {
    fn partial_cmp(&self, other: &*mut T) -> Option<Ordering> {
        (*self as *const u8 as usize).partial_cmp(&(*other as *const u8 as usize))
    }
}

impl<T: ?Sized> Ord for *mut T {
    fn cmp(&self, other: &*mut T) -> Ordering {
        (*self as *const u8 as usize).cmp(&(*other as *const u8 as usize))
    }
}
