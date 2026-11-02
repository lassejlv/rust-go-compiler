//! Reader-writer locks.

use super::poison::{map_result, LockResult, PoisonError, TryLockError, TryLockResult};
use super::sys::{futex_wait, futex_wake_all, panicking};
use crate::cell::UnsafeCell;
use crate::fmt;
use crate::ops::{Deref, DerefMut};
use crate::sync::atomic::{AtomicBool, AtomicIsize, AtomicU32, Ordering::SeqCst};

/// A reader-writer lock.
pub struct RwLock<T: ?Sized> {
    // > 0: number of readers; -1: write-locked; 0: unlocked
    state: AtomicIsize,
    seq: AtomicU32,
    poison: AtomicBool,
    data: UnsafeCell<T>,
}

impl<T> RwLock<T> {
    /// Creates a new instance of an `RwLock<T>` which is unlocked.
    #[inline]
    pub const fn new(t: T) -> RwLock<T> {
        RwLock { state: AtomicIsize::new(0), seq: AtomicU32::new(0), poison: AtomicBool::new(false), data: UnsafeCell::new(t) }
    }

    /// Consumes this `RwLock`, returning the underlying data.
    pub fn into_inner(self) -> LockResult<T> {
        let poisoned = self.poison.load(SeqCst);
        map_result(poisoned, self.data.into_inner())
    }
}

impl<T: ?Sized> RwLock<T> {
    fn try_read_raw(&self) -> bool {
        let s = self.state.load(SeqCst);
        s >= 0 && self.state.compare_exchange(s, s + 1, SeqCst, SeqCst).is_ok()
    }

    fn read_raw(&self) {
        loop {
            let s = self.state.load(SeqCst);
            if s >= 0 {
                if self.state.compare_exchange_weak(s, s + 1, SeqCst, SeqCst).is_ok() {
                    return;
                }
                continue;
            }
            let q = self.seq.load(SeqCst);
            if self.state.load(SeqCst) < 0 {
                futex_wait(&self.seq, q, u64::MAX);
            }
        }
    }

    fn write_raw(&self) {
        loop {
            if self.state.compare_exchange(0, -1, SeqCst, SeqCst).is_ok() {
                return;
            }
            let q = self.seq.load(SeqCst);
            if self.state.load(SeqCst) != 0 {
                futex_wait(&self.seq, q, u64::MAX);
            }
        }
    }

    fn read_unlock(&self) {
        if self.state.fetch_sub(1, SeqCst) == 1 {
            self.seq.fetch_add(1, SeqCst);
            futex_wake_all(&self.seq);
        }
    }

    fn write_unlock(&self) {
        self.state.store(0, SeqCst);
        self.seq.fetch_add(1, SeqCst);
        futex_wake_all(&self.seq);
    }

    /// Locks this `RwLock` with shared read access, blocking the current thread until it can be acquired.
    pub fn read(&self) -> LockResult<RwLockReadGuard<'_, T>> {
        self.read_raw();
        map_result(self.poison.load(SeqCst), RwLockReadGuard { lock: self })
    }

    /// Attempts to acquire this `RwLock` with shared read access.
    pub fn try_read(&self) -> TryLockResult<RwLockReadGuard<'_, T>> {
        if self.try_read_raw() {
            let g = RwLockReadGuard { lock: self };
            if self.poison.load(SeqCst) {
                Err(TryLockError::Poisoned(PoisonError::new(g)))
            } else {
                Ok(g)
            }
        } else {
            Err(TryLockError::WouldBlock)
        }
    }

    /// Locks this `RwLock` with exclusive write access, blocking the current thread until it can be acquired.
    pub fn write(&self) -> LockResult<RwLockWriteGuard<'_, T>> {
        self.write_raw();
        map_result(self.poison.load(SeqCst), RwLockWriteGuard { lock: self, panicking: panicking() })
    }

    /// Attempts to lock this `RwLock` with exclusive write access.
    pub fn try_write(&self) -> TryLockResult<RwLockWriteGuard<'_, T>> {
        if self.state.compare_exchange(0, -1, SeqCst, SeqCst).is_ok() {
            let g = RwLockWriteGuard { lock: self, panicking: panicking() };
            if self.poison.load(SeqCst) {
                Err(TryLockError::Poisoned(PoisonError::new(g)))
            } else {
                Ok(g)
            }
        } else {
            Err(TryLockError::WouldBlock)
        }
    }

    /// Returns a mutable reference to the underlying data.
    pub fn get_mut(&mut self) -> LockResult<&mut T> {
        let poisoned = self.poison.load(SeqCst);
        map_result(poisoned, self.data.get_mut())
    }

    /// Determines whether the lock is poisoned.
    #[inline]
    pub fn is_poisoned(&self) -> bool {
        self.poison.load(SeqCst)
    }

    /// Clear the poisoned state from a lock.
    #[inline]
    pub fn clear_poison(&self) {
        self.poison.store(false, SeqCst);
    }
}

impl<T: Default> Default for RwLock<T> {
    fn default() -> RwLock<T> {
        RwLock::new(Default::default())
    }
}

impl<T> From<T> for RwLock<T> {
    fn from(t: T) -> Self {
        RwLock::new(t)
    }
}

impl<T: ?Sized + fmt::Debug> fmt::Debug for RwLock<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut d = f.debug_struct("RwLock");
        match self.try_read() {
            Ok(guard) => {
                d.field("data", &&*guard);
            }
            Err(TryLockError::Poisoned(err)) => {
                d.field("data", &&**err.get_ref());
            }
            Err(TryLockError::WouldBlock) => {
                d.field("data", &format_args!("<locked>"));
            }
        }
        d.field("poisoned", &self.poison.load(SeqCst));
        d.finish_non_exhaustive()
    }
}

/// RAII structure used to release the shared read access of a lock when dropped.
pub struct RwLockReadGuard<'a, T: ?Sized + 'a> {
    lock: &'a RwLock<T>,
}

impl<'a, T: ?Sized> Deref for RwLockReadGuard<'a, T> {
    type Target = T;
    #[inline]
    fn deref(&self) -> &T {
        unsafe { &*self.lock.data.get() }
    }
}

impl<'a, T: ?Sized> Drop for RwLockReadGuard<'a, T> {
    fn drop(&mut self) {
        self.lock.read_unlock();
    }
}

impl<'a, T: ?Sized + fmt::Debug> fmt::Debug for RwLockReadGuard<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&**self, f)
    }
}

impl<'a, T: ?Sized + fmt::Display> fmt::Display for RwLockReadGuard<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&**self, f)
    }
}

/// RAII structure used to release the exclusive write access of a lock when dropped.
pub struct RwLockWriteGuard<'a, T: ?Sized + 'a> {
    lock: &'a RwLock<T>,
    panicking: bool,
}

impl<'a, T: ?Sized> Deref for RwLockWriteGuard<'a, T> {
    type Target = T;
    #[inline]
    fn deref(&self) -> &T {
        unsafe { &*self.lock.data.get() }
    }
}

impl<'a, T: ?Sized> DerefMut for RwLockWriteGuard<'a, T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.lock.data.get() }
    }
}

impl<'a, T: ?Sized> Drop for RwLockWriteGuard<'a, T> {
    fn drop(&mut self) {
        if !self.panicking && panicking() {
            self.lock.poison.store(true, SeqCst);
        }
        self.lock.write_unlock();
    }
}

impl<'a, T: ?Sized + fmt::Debug> fmt::Debug for RwLockWriteGuard<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&**self, f)
    }
}

impl<'a, T: ?Sized + fmt::Display> fmt::Display for RwLockWriteGuard<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&**self, f)
    }
}
