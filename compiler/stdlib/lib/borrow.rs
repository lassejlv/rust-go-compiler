//! A module for working with borrowed data.

use crate::cmp::Ordering;
use crate::fmt;
use crate::hash::{Hash, Hasher};
use crate::ops::{Add, AddAssign, Deref};

/// A trait for borrowing data.
pub trait Borrow<Borrowed: ?Sized> {
    /// Immutably borrows from an owned value.
    fn borrow(&self) -> &Borrowed;
}

/// A trait for mutably borrowing data.
pub trait BorrowMut<Borrowed: ?Sized>: Borrow<Borrowed> {
    /// Mutably borrows from an owned value.
    fn borrow_mut(&mut self) -> &mut Borrowed;
}

impl<T: ?Sized> Borrow<T> for T {
    #[inline]
    fn borrow(&self) -> &T {
        self
    }
}

impl<T: ?Sized> BorrowMut<T> for T {
    #[inline]
    fn borrow_mut(&mut self) -> &mut T {
        self
    }
}

impl<T: ?Sized> Borrow<T> for &T {
    #[inline]
    fn borrow(&self) -> &T {
        &**self
    }
}

impl<T: ?Sized> Borrow<T> for &mut T {
    #[inline]
    fn borrow(&self) -> &T {
        &**self
    }
}

impl<T: ?Sized> BorrowMut<T> for &mut T {
    #[inline]
    fn borrow_mut(&mut self) -> &mut T {
        &mut **self
    }
}

impl Borrow<str> for String {
    #[inline]
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

impl BorrowMut<str> for String {
    #[inline]
    fn borrow_mut(&mut self) -> &mut str {
        self.as_mut_str()
    }
}

impl<T> Borrow<[T]> for Vec<T> {
    #[inline]
    fn borrow(&self) -> &[T] {
        self.as_slice()
    }
}

impl<T> BorrowMut<[T]> for Vec<T> {
    #[inline]
    fn borrow_mut(&mut self) -> &mut [T] {
        self.as_mut_slice()
    }
}

impl<T, const N: usize> Borrow<[T]> for [T; N] {
    #[inline]
    fn borrow(&self) -> &[T] {
        self.as_slice()
    }
}

impl<T, const N: usize> BorrowMut<[T]> for [T; N] {
    #[inline]
    fn borrow_mut(&mut self) -> &mut [T] {
        self.as_mut_slice()
    }
}

/// A generalization of `Clone` to borrowed data.
pub trait ToOwned {
    /// The resulting type after obtaining ownership.
    type Owned: Borrow<Self>;

    /// Creates owned data from borrowed data, usually by cloning.
    fn to_owned(&self) -> Self::Owned;

    /// Uses borrowed data to replace owned data, usually by cloning.
    fn clone_into(&self, target: &mut Self::Owned) {
        *target = self.to_owned();
    }
}

impl<T: Clone> ToOwned for T {
    type Owned = T;
    #[inline]
    fn to_owned(&self) -> T {
        self.clone()
    }
}

impl ToOwned for str {
    type Owned = String;
    #[inline]
    fn to_owned(&self) -> String {
        String::from(self)
    }
}

impl<T: Clone> ToOwned for [T] {
    type Owned = Vec<T>;
    #[inline]
    fn to_owned(&self) -> Vec<T> {
        self.to_vec()
    }
}

/// A clone-on-write smart pointer.
pub enum Cow<'a, B: ?Sized + 'a>
where
    B: ToOwned,
{
    /// Borrowed data.
    Borrowed(&'a B),
    /// Owned data.
    Owned(<B as ToOwned>::Owned),
}

pub use self::Cow::{Borrowed, Owned};

impl<'a, B: ?Sized + ToOwned> Cow<'a, B> {
    /// Returns true if the data is borrowed.
    #[inline]
    pub fn is_borrowed(&self) -> bool {
        match *self {
            Cow::Borrowed(_) => true,
            Cow::Owned(_) => false,
        }
    }

    /// Returns true if the data is owned.
    #[inline]
    pub fn is_owned(&self) -> bool {
        !self.is_borrowed()
    }

    /// Acquires a mutable reference to the owned form of the data.
    pub fn to_mut(&mut self) -> &mut <B as ToOwned>::Owned {
        if let Cow::Borrowed(borrowed) = *self {
            *self = Cow::Owned(borrowed.to_owned());
        }
        match self {
            Cow::Borrowed(_) => unreachable!(),
            Cow::Owned(owned) => owned,
        }
    }

    /// Extracts the owned data.
    pub fn into_owned(self) -> <B as ToOwned>::Owned {
        match self {
            Cow::Borrowed(borrowed) => borrowed.to_owned(),
            Cow::Owned(owned) => owned,
        }
    }
}

impl<'a, B: ?Sized + ToOwned> Deref for Cow<'a, B> {
    type Target = B;
    fn deref(&self) -> &B {
        match *self {
            Cow::Borrowed(borrowed) => borrowed,
            Cow::Owned(ref owned) => owned.borrow(),
        }
    }
}

impl<'a, B: ?Sized + ToOwned> Clone for Cow<'a, B> {
    fn clone(&self) -> Self {
        match *self {
            Cow::Borrowed(b) => Cow::Borrowed(b),
            Cow::Owned(ref o) => {
                let b: &B = o.borrow();
                Cow::Owned(b.to_owned())
            }
        }
    }
}

impl<'a, B: ?Sized + ToOwned> Borrow<B> for Cow<'a, B> {
    fn borrow(&self) -> &B {
        &**self
    }
}

impl<'a, B: ?Sized + ToOwned> AsRef<B> for Cow<'a, B> {
    fn as_ref(&self) -> &B {
        &**self
    }
}

impl<'a, B: ?Sized + ToOwned + fmt::Debug> fmt::Debug for Cow<'a, B>
where
    B::Owned: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Cow::Borrowed(ref b) => fmt::Debug::fmt(b, f),
            Cow::Owned(ref o) => fmt::Debug::fmt(o, f),
        }
    }
}

impl<'a, B: ?Sized + ToOwned + fmt::Display> fmt::Display for Cow<'a, B>
where
    B::Owned: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Cow::Borrowed(ref b) => fmt::Display::fmt(b, f),
            Cow::Owned(ref o) => fmt::Display::fmt(o, f),
        }
    }
}

impl<'a, B: ?Sized + ToOwned> Default for Cow<'a, B>
where
    B::Owned: Default,
{
    fn default() -> Self {
        Cow::Owned(<B::Owned as Default>::default())
    }
}

impl<'a, B: ?Sized + ToOwned + Hash> Hash for Cow<'a, B> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        Hash::hash(&**self, state)
    }
}

impl<'a, 'b, B: ?Sized + ToOwned + PartialEq<C>, C: ?Sized + ToOwned> PartialEq<Cow<'b, C>> for Cow<'a, B> {
    #[inline]
    fn eq(&self, other: &Cow<'b, C>) -> bool {
        PartialEq::eq(&**self, &**other)
    }
}

impl<'a, B: ?Sized + ToOwned + Eq> Eq for Cow<'a, B> {}

impl<'a, B: ?Sized + ToOwned + PartialOrd> PartialOrd for Cow<'a, B> {
    #[inline]
    fn partial_cmp(&self, other: &Cow<'a, B>) -> Option<Ordering> {
        PartialOrd::partial_cmp(&**self, &**other)
    }
}

impl<'a, B: ?Sized + ToOwned + Ord> Ord for Cow<'a, B> {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        Ord::cmp(&**self, &**other)
    }
}

impl<'a> Add<&'a str> for Cow<'a, str> {
    type Output = Cow<'a, str>;
    #[inline]
    fn add(mut self, rhs: &'a str) -> Self::Output {
        self += rhs;
        self
    }
}

impl<'a> AddAssign<&'a str> for Cow<'a, str> {
    fn add_assign(&mut self, rhs: &'a str) {
        if self.is_empty() {
            *self = Cow::Borrowed(rhs)
        } else if !rhs.is_empty() {
            if let Cow::Borrowed(lhs) = *self {
                let mut s = String::with_capacity(lhs.len() + rhs.len());
                s.push_str(lhs);
                *self = Cow::Owned(s);
            }
            self.to_mut().push_str(rhs);
        }
    }
}

impl<'a, T: Clone> From<&'a [T]> for Cow<'a, [T]> {
    fn from(s: &'a [T]) -> Cow<'a, [T]> {
        Cow::Borrowed(s)
    }
}

impl<'a, T: Clone> From<Vec<T>> for Cow<'a, [T]> {
    fn from(v: Vec<T>) -> Cow<'a, [T]> {
        Cow::Owned(v)
    }
}

impl<'a, T: Clone> From<&'a Vec<T>> for Cow<'a, [T]> {
    fn from(v: &'a Vec<T>) -> Cow<'a, [T]> {
        Cow::Borrowed(v.as_slice())
    }
}
