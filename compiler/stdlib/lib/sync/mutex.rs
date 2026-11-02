//! Mutual exclusion.

use super::poison::{map_result, LockResult, PoisonError, TryLockError, TryLockResult};
use super::sys::{panicking, RawMutex};
use crate::cell::UnsafeCell;
use crate::fmt;
use crate::marker::PhantomData;
use crate::ops::{Deref, DerefMut};
use crate::sync::atomic::{AtomicBool, Ordering::SeqCst};

/// A mutual exclusion primitive useful for protecting shared data.
pub struct Mutex<T: ?Sized> {
    pub(crate) inner: RawMutex,
    pub(crate) poison: AtomicBool,
    data: UnsafeCell<T>,
}

impl<T> Mutex<T> {
    /// Creates a new mutex in an unlocked state ready for use.
    #[inline]
    pub const fn new(t: T) -> Mutex<T> {
        Mutex { inner: RawMutex::new(), poison: AtomicBool::new(false), data: UnsafeCell::new(t) }
    }

    /// Consumes this mutex, returning the underlying data.
    pub fn into_inner(self) -> LockResult<T> {
        let poisoned = self.poison.load(SeqCst);
        map_result(poisoned, self.data.into_inner())
    }
}

impl<T: ?Sized> Mutex<T> {
    /// Acquires a mutex, blocking the current thread until it is able to do so.
    pub fn lock(&self) -> LockResult<MutexGuard<'_, T>> {
        self.inner.lock();
        let guard = MutexGuard { lock: self, panicking: panicking(), _marker: PhantomData };
        map_result(self.poison.load(SeqCst), guard)
    }

    /// Attempts to acquire this lock.
    pub fn try_lock(&self) -> TryLockResult<MutexGuard<'_, T>> {
        if self.inner.try_lock() {
            let guard = MutexGuard { lock: self, panicking: panicking(), _marker: PhantomData };
            if self.poison.load(SeqCst) {
                Err(TryLockError::Poisoned(PoisonError::new(guard)))
            } else {
                Ok(guard)
            }
        } else {
            Err(TryLockError::WouldBlock)
        }
    }

    /// Determines whether the mutex is poisoned.
    #[inline]
    pub fn is_poisoned(&self) -> bool {
        self.poison.load(SeqCst)
    }

    /// Clear the poisoned state from a mutex.
    #[inline]
    pub fn clear_poison(&self) {
        self.poison.store(false, SeqCst);
    }

    /// Returns a mutable reference to the underlying data.
    pub fn get_mut(&mut self) -> LockResult<&mut T> {
        let poisoned = self.poison.load(SeqCst);
        map_result(poisoned, self.data.get_mut())
    }

    #[inline]
    pub(crate) fn data_ptr(&self) -> *mut T {
        self.data.get()
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
        match self.try_lock() {
            Ok(guard) => {
                d.field("data", &&*guard);
            }
            Err(TryLockError::Poisoned(err)) => {
                d.field("data", &&**err.get_ref());
            }
            Err(TryLockError::WouldBlock) => {
                d.field("data", &"<locked>");
            }
        }
        d.field("poisoned", &self.poison.load(SeqCst));
        d.finish_non_exhaustive()
    }
}

/// An RAII implementation of a "scoped lock" of a mutex.
pub struct MutexGuard<'a, T: ?Sized + 'a> {
    pub(crate) lock: &'a Mutex<T>,
    pub(crate) panicking: bool,
    _marker: PhantomData<&'a mut T>,
}

impl<'a, T: ?Sized> Deref for MutexGuard<'a, T> {
    type Target = T;
    #[inline]
    fn deref(&self) -> &T {
        unsafe { &*self.lock.data.get() }
    }
}

impl<'a, T: ?Sized> DerefMut for MutexGuard<'a, T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.lock.data.get() }
    }
}

impl<'a, T: ?Sized> Drop for MutexGuard<'a, T> {
    #[inline]
    fn drop(&mut self) {
        if !self.panicking && panicking() {
            self.lock.poison.store(true, SeqCst);
        }
        self.lock.inner.unlock();
    }
}

impl<'a, T: ?Sized + fmt::Debug> fmt::Debug for MutexGuard<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&**self, f)
    }
}

impl<'a, T: ?Sized + fmt::Display> fmt::Display for MutexGuard<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&**self, f)
    }
}
