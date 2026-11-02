//! `ControlFlow`: whether an operation should exit early or continue.

/// Used to tell an operation whether it should exit early or go on as usual.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ControlFlow<B, C = ()> {
    /// Move on to the next phase of the operation as normal.
    Continue(C),
    /// Exit the operation without running subsequent phases.
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
}
