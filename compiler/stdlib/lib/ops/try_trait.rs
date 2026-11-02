//! The `Try` trait (used by `Iterator::try_fold` and friends; the `?`
//! operator itself is built into the compiler).

use super::ControlFlow;

/// The `?` operator and `try {}` blocks.
pub trait Try {
    /// The type of the value produced by `?` when *not* short-circuiting.
    type Output;
    /// The type of the value passed to `from_residual` when short-circuiting.
    type Residual;
    /// Constructs the type from its `Output` type.
    fn from_output(output: Self::Output) -> Self;
    /// Constructs the type from a compatible `Residual` type.
    fn from_residual(residual: Self::Residual) -> Self;
    /// Used in `?` to decide whether the operator should produce a value
    /// (`Continue`) or propagate a value back to the caller (`Break`).
    fn branch(self) -> ControlFlow<Self::Residual, Self::Output>;
}

/// Used to specify which residuals can be converted into which `Try` types.
pub trait FromResidual<R> {
    /// Constructs the type from a compatible `Residual` type.
    fn from_residual(residual: R) -> Self;
}

impl<T> FromResidual<Option<crate::convert::Infallible>> for Option<T> {
    #[inline]
    fn from_residual(_residual: Option<crate::convert::Infallible>) -> Self {
        None
    }
}

impl<T, E, F: From<E>> FromResidual<Result<crate::convert::Infallible, E>> for Result<T, F> {
    #[inline]
    fn from_residual(residual: Result<crate::convert::Infallible, E>) -> Self {
        match residual {
            Err(e) => Err(From::from(e)),
            Ok(never) => match never {},
        }
    }
}

impl<B, C> FromResidual<ControlFlow<B, crate::convert::Infallible>> for ControlFlow<B, C> {
    #[inline]
    fn from_residual(residual: ControlFlow<B, crate::convert::Infallible>) -> Self {
        match residual {
            ControlFlow::Break(b) => ControlFlow::Break(b),
            ControlFlow::Continue(never) => match never {},
        }
    }
}

impl<T> Try for Option<T> {
    type Output = T;
    type Residual = Option<crate::convert::Infallible>;
    #[inline]
    fn from_output(output: T) -> Self {
        Some(output)
    }
    #[inline]
    fn from_residual(_residual: Option<crate::convert::Infallible>) -> Self {
        None
    }
    #[inline]
    fn branch(self) -> ControlFlow<Option<crate::convert::Infallible>, T> {
        match self {
            Some(v) => ControlFlow::Continue(v),
            None => ControlFlow::Break(None),
        }
    }
}

impl<T, E> Try for Result<T, E> {
    type Output = T;
    type Residual = Result<crate::convert::Infallible, E>;
    #[inline]
    fn from_output(output: T) -> Self {
        Ok(output)
    }
    #[inline]
    fn from_residual(residual: Result<crate::convert::Infallible, E>) -> Self {
        match residual {
            Err(e) => Err(e),
            Ok(never) => match never {},
        }
    }
    #[inline]
    fn branch(self) -> ControlFlow<Result<crate::convert::Infallible, E>, T> {
        match self {
            Ok(v) => ControlFlow::Continue(v),
            Err(e) => ControlFlow::Break(Err(e)),
        }
    }
}

impl<B, C> Try for ControlFlow<B, C> {
    type Output = C;
    type Residual = ControlFlow<B, crate::convert::Infallible>;
    #[inline]
    fn from_output(output: C) -> Self {
        ControlFlow::Continue(output)
    }
    #[inline]
    fn from_residual(residual: ControlFlow<B, crate::convert::Infallible>) -> Self {
        match residual {
            ControlFlow::Break(b) => ControlFlow::Break(b),
            ControlFlow::Continue(never) => match never {},
        }
    }
    #[inline]
    fn branch(self) -> ControlFlow<ControlFlow<B, crate::convert::Infallible>, C> {
        match self {
            ControlFlow::Continue(c) => ControlFlow::Continue(c),
            ControlFlow::Break(b) => ControlFlow::Break(ControlFlow::Break(b)),
        }
    }
}
