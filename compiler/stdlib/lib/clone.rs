//! The `Clone` trait.

/// A common trait for the ability to explicitly duplicate an object.
#[lang = "clone"]
pub trait Clone: Sized {
    /// Returns a copy of the value.
    fn clone(&self) -> Self;

    /// Performs copy-assignment from `source`.
    fn clone_from(&mut self, source: &Self) {
        *self = source.clone()
    }
}

macro_rules! __std_clone_by_copy {
    ($($t:ty)*) => {
        $(
            impl Clone for $t {
                #[inline]
                fn clone(&self) -> Self {
                    *self
                }
            }
        )*
    };
}

__std_clone_by_copy! { i8 i16 i32 i64 i128 isize u8 u16 u32 u64 u128 usize f32 f64 bool char () }

impl<T: ?Sized> Clone for &T {
    #[inline]
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: ?Sized> Clone for *const T {
    #[inline]
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: ?Sized> Clone for *mut T {
    #[inline]
    fn clone(&self) -> Self {
        *self
    }
}
