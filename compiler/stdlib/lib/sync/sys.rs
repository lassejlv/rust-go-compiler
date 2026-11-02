//! Low-level futex-based locking primitives.

use crate::intrinsics;
use crate::sync::atomic::{AtomicU32, Ordering::SeqCst};

/// Waits while `*a == expected` (or until woken / the timeout in
/// nanoseconds elapses; `u64::MAX` = no timeout). Returns false on timeout.
#[inline]
pub(crate) fn futex_wait(a: &AtomicU32, expected: u32, timeout_ns: u64) -> bool {
    unsafe { intrinsics::futex_wait(a.as_ptr() as *const u32, expected, timeout_ns) }
}

/// Wakes one waiter.
#[inline]
pub(crate) fn futex_wake(a: &AtomicU32) -> bool {
    unsafe { intrinsics::futex_wake(a.as_ptr() as *const u32) }
}

/// Wakes all waiters.
#[inline]
pub(crate) fn futex_wake_all(a: &AtomicU32) {
    unsafe { intrinsics::futex_wake_all(a.as_ptr() as *const u32) }
}

/// Whether the current thread is unwinding because of a panic.
#[inline]
pub(crate) fn panicking() -> bool {
    unsafe { intrinsics::thread_panicking() }
}

/// A futex mutex: 0 = unlocked, 1 = locked, 2 = locked with waiters.
pub(crate) struct RawMutex {
    state: AtomicU32,
}

impl RawMutex {
    #[inline]
    pub(crate) const fn new() -> RawMutex {
        RawMutex { state: AtomicU32::new(0) }
    }

    #[inline]
    pub(crate) fn try_lock(&self) -> bool {
        self.state.compare_exchange(0, 1, SeqCst, SeqCst).is_ok()
    }

    #[inline]
    pub(crate) fn lock(&self) {
        if self.state.compare_exchange(0, 1, SeqCst, SeqCst).is_err() {
            self.lock_contended();
        }
    }

    fn spin(&self) -> u32 {
        let mut n = 100;
        loop {
            let s = self.state.load(SeqCst);
            if s != 1 || n == 0 {
                return s;
            }
            n -= 1;
        }
    }

    fn lock_contended(&self) {
        let mut s = self.spin();
        if s == 0 {
            match self.state.compare_exchange(0, 1, SeqCst, SeqCst) {
                Ok(_) => return,
                Err(x) => s = x,
            }
        }
        loop {
            if s != 2 && self.state.swap(2, SeqCst) == 0 {
                return;
            }
            futex_wait(&self.state, 2, u64::MAX);
            s = self.spin();
        }
    }

    #[inline]
    pub(crate) fn unlock(&self) {
        if self.state.swap(0, SeqCst) == 2 {
            futex_wake(&self.state);
        }
    }

    #[inline]
    pub(crate) fn is_locked(&self) -> bool {
        self.state.load(SeqCst) != 0
    }
}

/// A futex condition variable (a sequence counter).
pub(crate) struct RawCondvar {
    seq: AtomicU32,
}

impl RawCondvar {
    #[inline]
    pub(crate) const fn new() -> RawCondvar {
        RawCondvar { seq: AtomicU32::new(0) }
    }

    /// Atomically unlocks `m`, waits, and relocks `m`. Returns false on timeout.
    pub(crate) fn wait(&self, m: &RawMutex, timeout_ns: u64) -> bool {
        let v = self.seq.load(SeqCst);
        m.unlock();
        let r = futex_wait(&self.seq, v, timeout_ns);
        m.lock();
        r
    }

    #[inline]
    pub(crate) fn notify_one(&self) {
        self.seq.fetch_add(1, SeqCst);
        futex_wake(&self.seq);
    }

    #[inline]
    pub(crate) fn notify_all(&self) {
        self.seq.fetch_add(1, SeqCst);
        futex_wake_all(&self.seq);
    }
}
