//! The `Fn` traits. Closures, function items and function pointers
//! implement them built into the compiler (as does `Box<F>`).

/// The version of the call operator that takes a by-value receiver.
#[lang = "fn_once"]
#[must_use]
pub trait FnOnce<Args> {
    /// The returned type after the call operator is used.
    type Output;
    /// Performs the call operation.
    fn call_once(self, args: Args) -> Self::Output;
}

/// The version of the call operator that takes a mutable receiver.
#[lang = "fn_mut"]
#[must_use]
pub trait FnMut<Args>: FnOnce<Args> {
    /// Performs the call operation.
    fn call_mut(&mut self, args: Args) -> Self::Output;
}

/// The version of the call operator that takes an immutable receiver.
#[lang = "fn"]
#[must_use]
pub trait Fn<Args>: FnMut<Args> {
    /// Performs the call operation.
    fn call(&self, args: Args) -> Self::Output;
}
