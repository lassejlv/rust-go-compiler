//! Inherent methods of the `bool` primitive type.

impl bool {
    /// Returns `Some(t)` if the `bool` is `true`, or `None` otherwise.
    #[inline]
    pub fn then_some<T>(self, t: T) -> Option<T> {
        if self { Some(t) } else { None }
    }

    /// Returns `Some(f())` if the `bool` is `true`, or `None` otherwise.
    #[inline]
    pub fn then<T, F: FnOnce() -> T>(self, f: F) -> Option<T> {
        if self { Some(f()) } else { None }
    }
}
