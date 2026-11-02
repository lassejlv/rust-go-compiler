//! `Deref` and `DerefMut`.

/// Used for immutable dereferencing operations, like `*v`.
#[lang = "deref"]
pub trait Deref {
    /// The resulting type after dereferencing.
    type Target: ?Sized;
    /// Dereferences the value.
    fn deref(&self) -> &Self::Target;
}

/// Used for mutable dereferencing operations, like in `*v = 1;`.
#[lang = "deref_mut"]
pub trait DerefMut: Deref {
    /// Mutably dereferences the value.
    fn deref_mut(&mut self) -> &mut Self::Target;
}

impl<T: ?Sized> Deref for &T {
    type Target = T;
    #[inline]
    fn deref(&self) -> &T {
        &**self
    }
}

impl<T: ?Sized> Deref for &mut T {
    type Target = T;
    #[inline]
    fn deref(&self) -> &T {
        &**self
    }
}

impl<T: ?Sized> DerefMut for &mut T {
    #[inline]
    fn deref_mut(&mut self) -> &mut T {
        &mut **self
    }
}
