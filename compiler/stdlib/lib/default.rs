//! The `Default` trait.

/// A trait for giving a type a useful default value.
pub trait Default: Sized {
    /// Returns the "default value" for a type.
    fn default() -> Self;
}

macro_rules! __std_default_impl {
    ($($t:ty => $v:expr),* $(,)?) => {
        $(
            impl Default for $t {
                #[inline]
                fn default() -> $t {
                    $v
                }
            }
        )*
    };
}

__std_default_impl! {
    () => (),
    bool => false,
    char => '\x00',
    i8 => 0, i16 => 0, i32 => 0, i64 => 0, i128 => 0, isize => 0,
    u8 => 0, u16 => 0, u32 => 0, u64 => 0, u128 => 0, usize => 0,
    f32 => 0.0, f64 => 0.0,
}

impl Default for &str {
    fn default() -> Self {
        ""
    }
}

impl<T> Default for &[T] {
    fn default() -> Self {
        &[]
    }
}

impl<T> Default for &mut [T] {
    fn default() -> Self {
        &mut []
    }
}
