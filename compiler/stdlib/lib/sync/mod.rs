//! Synchronization primitives. This implementation is single-threaded:
//! `Arc` is not atomic and locks never block (a contended lock panics).

mod arc;
pub mod atomic;

pub use self::arc::{Arc, Weak};

use crate::cell::{Cell, OnceCell, UnsafeCell};
use crate::fmt;
use crate::marker::PhantomData;
use crate::ops::{Deref, DerefMut};

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

/// An enumeration of possible errors associated with a `TryLockResult`.
pub enum TryLockError<T> {
    /// The lock could not be acquired because another thread failed while holding the lock.
    Poisoned(PoisonError<T>),
    /// The lock could not be acquired at this time because the operation would otherwise block.
    WouldBlock,
}

impl<T> fmt::Debug for TryLockError<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            TryLockError::Poisoned(..) => f.write_str("Poisoned(..)"),
            TryLockError::WouldBlock => f.write_str("WouldBlock"),
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

/// A type alias for the result of a lock method which can be poisoned.
pub type LockResult<Guard> = Result<Guard, PoisonError<Guard>>;

/// A type alias for the result of a nonblocking locking method.
pub type TryLockResult<Guard> = Result<Guard, TryLockError<Guard>>;

// ---------------------------------------------------------------------------
// Mutex

/// A mutual exclusion primitive useful for protecting shared data.
pub struct Mutex<T: ?Sized> {
    locked: Cell<bool>,
    data: UnsafeCell<T>,
}

impl<T> Mutex<T> {
    /// Creates a new mutex in an unlocked state ready for use.
    #[inline]
    pub const fn new(t: T) -> Mutex<T> {
        Mutex { locked: Cell::new(false), data: UnsafeCell::new(t) }
    }

    /// Consumes this mutex, returning the underlying data.
    pub fn into_inner(self) -> LockResult<T> {
        Ok(self.data.into_inner())
    }
}

impl<T: ?Sized> Mutex<T> {
    /// Acquires a mutex.
    #[track_caller]
    pub fn lock(&self) -> LockResult<MutexGuard<'_, T>> {
        if self.locked.get() {
            crate::rt::panic_str("deadlock: Mutex is already locked by the current thread", file!(), line!(), column!());
        }
        self.locked.set(true);
        Ok(MutexGuard { lock: self, _marker: PhantomData })
    }

    /// Attempts to acquire this lock.
    pub fn try_lock(&self) -> TryLockResult<MutexGuard<'_, T>> {
        if self.locked.get() {
            Err(TryLockError::WouldBlock)
        } else {
            self.locked.set(true);
            Ok(MutexGuard { lock: self, _marker: PhantomData })
        }
    }

    /// Determines whether the mutex is poisoned.
    pub fn is_poisoned(&self) -> bool {
        false
    }

    /// Clear the poisoned state from a mutex.
    pub fn clear_poison(&self) {}

    /// Returns a mutable reference to the underlying data.
    pub fn get_mut(&mut self) -> LockResult<&mut T> {
        Ok(self.data.get_mut())
    }
}

impl<T: Default> Default for Mutex<T> {
    fn default() -> Mutex<T> {
        Mutex::new(Default::default())
    }
}

impl<T> From<T> for Mutex<T> {
    fn from(t: T) -> Self {
        Mutex::new(t)
    }
}

impl<T: ?Sized + fmt::Debug> fmt::Debug for Mutex<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut d = f.debug_struct("Mutex");
        if self.locked.get() {
            d.field("data", &format_args!("<locked>"));
        } else {
            d.field("data", unsafe { &&*self.data.get() });
        }
        d.field("poisoned", &false);
        d.finish_non_exhaustive()
    }
}

/// An RAII implementation of a "scoped lock" of a mutex.
pub struct MutexGuard<'a, T: ?Sized + 'a> {
    lock: &'a Mutex<T>,
    _marker: PhantomData<&'a mut T>,
}

impl<'a, T: ?Sized> Deref for MutexGuard<'a, T> {
    type Target = T;
    fn deref(&self) -> &T {
        unsafe { &*self.lock.data.get() }
    }
}

impl<'a, T: ?Sized> DerefMut for MutexGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.lock.data.get() }
    }
}

impl<'a, T: ?Sized> Drop for MutexGuard<'a, T> {
    #[inline]
    fn drop(&mut self) {
        self.lock.locked.set(false);
    }
}

impl<'a, T: ?Sized + fmt::Debug> fmt::Debug for MutexGuard<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&**self, f)
    }
}

impl<'a, T: ?Sized + fmt::Display> fmt::Display for MutexGuard<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        (**self).fmt(f)
    }
}

// ---------------------------------------------------------------------------
// RwLock

/// A reader-writer lock.
pub struct RwLock<T: ?Sized> {
    state: Cell<isize>,
    data: UnsafeCell<T>,
}

impl<T> RwLock<T> {
    /// Creates a new instance of an `RwLock<T>` which is unlocked.
    #[inline]
    pub const fn new(t: T) -> RwLock<T> {
        RwLock { state: Cell::new(0), data: UnsafeCell::new(t) }
    }

    /// Consumes this `RwLock`, returning the underlying data.
    pub fn into_inner(self) -> LockResult<T> {
        Ok(self.data.into_inner())
    }
}

impl<T: ?Sized> RwLock<T> {
    /// Locks this `RwLock` with shared read access.
    #[track_caller]
    pub fn read(&self) -> LockResult<RwLockReadGuard<'_, T>> {
        let s = self.state.get();
        if s < 0 {
            crate::rt::panic_str("rwlock read lock would result in deadlock", file!(), line!(), column!());
        }
        self.state.set(s + 1);
        Ok(RwLockReadGuard { lock: self })
    }

    /// Locks this `RwLock` with exclusive write access.
    #[track_caller]
    pub fn write(&self) -> LockResult<RwLockWriteGuard<'_, T>> {
        if self.state.get() != 0 {
            crate::rt::panic_str("rwlock write lock would result in deadlock", file!(), line!(), column!());
        }
        self.state.set(-1);
        Ok(RwLockWriteGuard { lock: self })
    }

    /// Returns a mutable reference to the underlying data.
    pub fn get_mut(&mut self) -> LockResult<&mut T> {
        Ok(self.data.get_mut())
    }

    /// Determines whether the lock is poisoned.
    pub fn is_poisoned(&self) -> bool {
        false
    }
}

impl<T: Default> Default for RwLock<T> {
    fn default() -> RwLock<T> {
        RwLock::new(Default::default())
    }
}

impl<T: ?Sized + fmt::Debug> fmt::Debug for RwLock<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut d = f.debug_struct("RwLock");
        if self.state.get() < 0 {
            d.field("data", &format_args!("<locked>"));
        } else {
            d.field("data", unsafe { &&*self.data.get() });
        }
        d.field("poisoned", &false);
        d.finish_non_exhaustive()
    }
}

/// RAII structure used to release the shared read access of a lock when dropped.
pub struct RwLockReadGuard<'a, T: ?Sized + 'a> {
    lock: &'a RwLock<T>,
}

impl<'a, T: ?Sized> Deref for RwLockReadGuard<'a, T> {
    type Target = T;
    fn deref(&self) -> &T {
        unsafe { &*self.lock.data.get() }
    }
}

impl<'a, T: ?Sized> Drop for RwLockReadGuard<'a, T> {
    fn drop(&mut self) {
        let s = self.lock.state.get();
        self.lock.state.set(s - 1);
    }
}

/// RAII structure used to release the exclusive write access of a lock when dropped.
pub struct RwLockWriteGuard<'a, T: ?Sized + 'a> {
    lock: &'a RwLock<T>,
}

impl<'a, T: ?Sized> Deref for RwLockWriteGuard<'a, T> {
    type Target = T;
    fn deref(&self) -> &T {
        unsafe { &*self.lock.data.get() }
    }
}

impl<'a, T: ?Sized> DerefMut for RwLockWriteGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.lock.data.get() }
    }
}

impl<'a, T: ?Sized> Drop for RwLockWriteGuard<'a, T> {
    fn drop(&mut self) {
        self.lock.state.set(0);
    }
}

// ---------------------------------------------------------------------------
// Once / OnceLock / LazyLock

/// A synchronization primitive which can be used to run a one-time global initialization.
pub struct Once {
    done: Cell<bool>,
}

impl Once {
    /// Creates a new `Once` value.
    pub const fn new() -> Once {
        Once { done: Cell::new(false) }
    }

    /// Performs an initialization routine once and only once.
    pub fn call_once<F: FnOnce()>(&self, f: F) {
        if !self.done.get() {
            self.done.set(true);
            f();
        }
    }

    /// Returns `true` if some `call_once()` call has completed successfully.
    pub fn is_completed(&self) -> bool {
        self.done.get()
    }
}

/// A synchronization primitive which can nominally be written to only once.
pub struct OnceLock<T> {
    cell: OnceCell<T>,
}

impl<T> OnceLock<T> {
    /// Creates a new uninitialized cell.
    pub const fn new() -> OnceLock<T> {
        OnceLock { cell: OnceCell::new() }
    }

    /// Gets the reference to the underlying value.
    pub fn get(&self) -> Option<&T> {
        self.cell.get()
    }

    /// Initializes the contents of the cell to `value`.
    pub fn set(&self, value: T) -> Result<(), T> {
        self.cell.set(value)
    }

    /// Gets the contents of the cell, initializing it to `f()` if the cell was uninitialized.
    pub fn get_or_init<F: FnOnce() -> T>(&self, f: F) -> &T {
        self.cell.get_or_init(f)
    }

    /// Consumes the `OnceLock`, returning the wrapped value.
    pub fn into_inner(self) -> Option<T> {
        self.cell.into_inner()
    }
}

impl<T> Default for OnceLock<T> {
    fn default() -> OnceLock<T> {
        OnceLock::new()
    }
}

impl<T: fmt::Debug> fmt::Debug for OnceLock<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut d = f.debug_tuple("OnceLock");
        match self.get() {
            Some(v) => d.field(v),
            None => d.field(&format_args!("<uninit>")),
        };
        d.finish()
    }
}

/// A value which is initialized on the first access.
pub struct LazyLock<T, F = fn() -> T> {
    inner: crate::cell::LazyCell<T, F>,
}

impl<T, F: FnOnce() -> T> LazyLock<T, F> {
    /// Creates a new lazy value with the given initializing function.
    pub const fn new(f: F) -> LazyLock<T, F> {
        LazyLock { inner: crate::cell::LazyCell::new(f) }
    }

    /// Forces the evaluation of this lazy value and returns a reference to result.
    pub fn force(this: &LazyLock<T, F>) -> &T {
        crate::cell::LazyCell::force(&this.inner)
    }
}

impl<T, F: FnOnce() -> T> Deref for LazyLock<T, F> {
    type Target = T;
    fn deref(&self) -> &T {
        LazyLock::force(self)
    }
}
