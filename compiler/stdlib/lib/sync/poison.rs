//! Lock poisoning.

use crate::fmt;

/// A type of error which can be returned whenever a lock is acquired.
pub struct PoisonError<T> {
    guard: T,
}

impl<T> PoisonError<T> {
    /// Creates a `PoisonError`.
    pub fn new(guard: T) -> PoisonError<T> {
        PoisonError { guard }
    }
    /// Consumes this error indicating that a lock is poisoned, returning the underlying guard.
    pub fn into_inner(self) -> T {
        self.guard
    }
    /// Reaches into this error, returning a reference to the underlying guard.
    pub fn get_ref(&self) -> &T {
        &self.guard
    }
    /// Reaches into this error, returning a mutable reference to the underlying guard.
    pub fn get_mut(&mut self) -> &mut T {
        &mut self.guard
    }
}

impl<T> fmt::Debug for PoisonError<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PoisonError").finish_non_exhaustive()
    }
}

impl<T> fmt::Display for PoisonError<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("poisoned lock: another task failed inside")
    }
}

impl<T> crate::error::Error for PoisonError<T> {}

/// An enumeration of possible errors associated with a `TryLockResult`.
pub enum TryLockError<T> {
    /// The lock could not be acquired because another thread failed while holding the lock.
    Poisoned(PoisonError<T>),
    /// The lock could not be acquired at this time because the operation would otherwise block.
    WouldBlock,
}

impl<T> From<PoisonError<T>> for TryLockError<T> {
    fn from(err: PoisonError<T>) -> TryLockError<T> {
        TryLockError::Poisoned(err)
    }
}

impl<T> fmt::Debug for TryLockError<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            TryLockError::Poisoned(..) => fmt::Debug::fmt("Poisoned(..)", f),
            TryLockError::WouldBlock => fmt::Debug::fmt("WouldBlock", f),
        }
    }
}

impl<T> fmt::Display for TryLockError<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            TryLockError::Poisoned(..) => f.write_str("poisoned lock: another task failed inside"),
            TryLockError::WouldBlock => f.write_str("try_lock failed because the operation would block"),
        }
    }
}

impl<T> crate::error::Error for TryLockError<T> {}

/// A type alias for the result of a lock method which can be poisoned.
pub type LockResult<Guard> = Result<Guard, PoisonError<Guard>>;

/// A type alias for the result of a nonblocking locking method.
pub type TryLockResult<Guard> = Result<Guard, TryLockError<Guard>>;

/// Wraps `guard` into a `LockResult` according to the poison flag.
#[inline]
pub(crate) fn map_result<G>(poisoned: bool, guard: G) -> LockResult<G> {
    if poisoned {
        Err(PoisonError::new(guard))
    } else {
        Ok(guard)
    }
}
