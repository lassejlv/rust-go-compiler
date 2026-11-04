//! `ControlFlow`: whether an operation should exit early or continue.

/// Used to tell an operation whether it should exit early or go on as usual.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[lang = "ControlFlow"]
pub enum ControlFlow<B, C = ()> {
    /// Move on to the next phase of the operation as normal.
    #[lang = "Continue"]
    Continue(C),
    /// Exit the operation without running subsequent phases.
    #[lang = "Break"]
    Break(B),
}

impl<B, C> ControlFlow<B, C> {
    /// Returns `true` if this is a `Break` variant.
    pub fn is_break(&self) -> bool {
        match *self {
            ControlFlow::Break(_) => true,
            ControlFlow::Continue(_) => false,
        }
    }
    /// Returns `true` if this is a `Continue` variant.
    pub fn is_continue(&self) -> bool {
        !self.is_break()
    }
    /// Converts the `ControlFlow` into an `Option` of the break value.
    pub fn break_value(self) -> Option<B> {
        match self {
            ControlFlow::Continue(..) => None,
            ControlFlow::Break(x) => Some(x),
        }
    }
    /// Converts the `ControlFlow` into an `Option` of the continue value.
    pub fn continue_value(self) -> Option<C> {
        match self {
            ControlFlow::Continue(x) => Some(x),
            ControlFlow::Break(..) => None,
        }
    }
    /// Converts the `ControlFlow` into a `Result`: `Ok` for `Break`, `Err` for `Continue`.
    pub fn break_ok(self) -> Result<B, C> {
        match self {
            ControlFlow::Continue(c) => Err(c),
            ControlFlow::Break(b) => Ok(b),
        }
    }
    /// Converts the `ControlFlow` into a `Result`: `Ok` for `Continue`, `Err` for `Break`.
    pub fn continue_ok(self) -> Result<C, B> {
        match self {
            ControlFlow::Continue(c) => Ok(c),
            ControlFlow::Break(b) => Err(b),
        }
    }
    /// Maps `ControlFlow<B, C>` to `ControlFlow<T, C>` by applying a function to the break value.
    pub fn map_break<T, F: FnOnce(B) -> T>(self, f: F) -> ControlFlow<T, C> {
        match self {
            ControlFlow::Continue(x) => ControlFlow::Continue(x),
            ControlFlow::Break(x) => ControlFlow::Break(f(x)),
        }
    }
    /// Maps `ControlFlow<B, C>` to `ControlFlow<B, T>` by applying a function to the continue value.
    pub fn map_continue<T, F: FnOnce(C) -> T>(self, f: F) -> ControlFlow<B, T> {
        match self {
            ControlFlow::Continue(x) => ControlFlow::Continue(f(x)),
            ControlFlow::Break(x) => ControlFlow::Break(x),
        }
    }
}
