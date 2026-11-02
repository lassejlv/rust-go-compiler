//! Types that pin data to a location in memory.

use crate::boxed::Box;
use crate::fmt;
use crate::marker::Unpin;
use crate::ops::{Deref, DerefMut};

/// A pointer which pins its pointee in place.
#[derive(Copy, Clone)]
#[rustc_coerce_unsized]
pub struct Pin<P> {
    #[doc(hidden)]
    pub __pointer: P,
}

impl<P: Deref> Pin<P> {
    /// Constructs a new `Pin` around a pointer to data whose type is `Unpin`.
    pub fn new(pointer: P) -> Pin<P>
    where
        P::Target: Unpin,
    {
        Pin { __pointer: pointer }
    }
    /// Constructs a new `Pin` without checking `Unpin`.
    pub unsafe fn new_unchecked(pointer: P) -> Pin<P> {
        Pin { __pointer: pointer }
    }
    /// Gets a pinned shared reference from this pinned pointer.
    pub fn as_ref(&self) -> Pin<&P::Target> {
        Pin { __pointer: &*self.__pointer }
    }
    /// Unwraps the pointer (target must be `Unpin`).
    pub fn into_inner(pin: Pin<P>) -> P
    where
        P::Target: Unpin,
    {
        pin.__pointer
    }
    /// Unwraps the pointer without checking `Unpin`.
    pub unsafe fn into_inner_unchecked(pin: Pin<P>) -> P {
        pin.__pointer
    }
}

impl<P: DerefMut> Pin<P> {
    /// Gets a pinned mutable reference from this pinned pointer.
    pub fn as_mut(&mut self) -> Pin<&mut P::Target> {
        Pin { __pointer: &mut *self.__pointer }
    }
    /// Assigns a new value to the memory behind the pinned reference.
    pub fn set(&mut self, value: P::Target)
    where
        P::Target: Sized,
    {
        *self.__pointer = value;
    }
}

impl<'a, T: ?Sized> Pin<&'a T> {
    pub fn get_ref(self) -> &'a T {
        self.__pointer
    }
    pub unsafe fn map_unchecked<U: ?Sized, F: FnOnce(&T) -> &U>(self, func: F) -> Pin<&'a U> {
        Pin { __pointer: func(self.__pointer) }
    }
}

impl<'a, T: ?Sized> Pin<&'a mut T> {
    pub fn into_ref(self) -> Pin<&'a T> {
        Pin { __pointer: self.__pointer }
    }
    pub fn get_mut(self) -> &'a mut T
    where
        T: Unpin,
    {
        self.__pointer
    }
    pub unsafe fn get_unchecked_mut(self) -> &'a mut T {
        self.__pointer
    }
    pub unsafe fn map_unchecked_mut<U: ?Sized, F: FnOnce(&mut T) -> &mut U>(self, func: F) -> Pin<&'a mut U> {
        Pin { __pointer: func(self.__pointer) }
    }
}

impl<P: Deref> Deref for Pin<P> {
    type Target = P::Target;
    fn deref(&self) -> &P::Target {
        &*self.__pointer
    }
}

impl<P: DerefMut> DerefMut for Pin<P> {
    fn deref_mut(&mut self) -> &mut P::Target {
        &mut *self.__pointer
    }
}

impl<P: fmt::Debug> fmt::Debug for Pin<P> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.__pointer, f)
    }
}

impl<P: fmt::Display> fmt::Display for Pin<P> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.__pointer, f)
    }
}

impl<T> Box<T> {
    /// Constructs a new `Pin<Box<T>>`.
    pub fn pin(x: T) -> Pin<Box<T>> {
        Pin { __pointer: Box::new(x) }
    }
}

impl<T: ?Sized> Box<T> {
    /// Converts a `Box<T>` into a `Pin<Box<T>>`.
    pub fn into_pin(boxed: Box<T>) -> Pin<Box<T>> {
        Pin { __pointer: boxed }
    }
}

impl<T: ?Sized> From<Box<T>> for Pin<Box<T>> {
    fn from(boxed: Box<T>) -> Pin<Box<T>> {
        Pin { __pointer: boxed }
    }
}
