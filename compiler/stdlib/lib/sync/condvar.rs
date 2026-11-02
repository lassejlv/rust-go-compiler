//! Condition variables.

use super::mutex::MutexGuard;
use super::poison::{map_result, LockResult, PoisonError};
use super::sys::RawCondvar;
use crate::fmt;
use crate::sync::atomic::Ordering::SeqCst;
use crate::time::{Duration, Instant};

/// A type indicating whether a timed wait on a condition variable returned due to a time out or not.
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub struct WaitTimeoutResult(bool);

impl WaitTimeoutResult {
    /// Returns `true` if the wait was known to have timed out.
    #[inline]
    pub fn timed_out(&self) -> bool {
        self.0
    }
}

/// A Condition Variable.
pub struct Condvar {
    inner: RawCondvar,
}

fn duration_ns(d: Duration) -> u64 {
    let n = d.as_nanos();
    if n >= (u64::MAX as u128) {
        u64::MAX - 1
    } else {
        n as u64
    }
}

impl Condvar {
    /// Creates a new condition variable which is ready to be waited on and notified.
    #[inline]
    pub const fn new() -> Condvar {
        Condvar { inner: RawCondvar::new() }
    }

    /// Blocks the current thread until this condition variable receives a notification.
    pub fn wait<'a, T>(&self, guard: MutexGuard<'a, T>) -> LockResult<MutexGuard<'a, T>> {
        let lock = guard.lock;
        self.inner.wait(&lock.inner, u64::MAX);
        map_result(lock.poison.load(SeqCst), guard)
    }

    /// Blocks the current thread until the provided condition becomes false.
    pub fn wait_while<'a, T, F>(&self, guard: MutexGuard<'a, T>, condition: F) -> LockResult<MutexGuard<'a, T>>
    where
        F: FnMut(&mut T) -> bool,
    {
        let mut condition = condition;
        let mut guard = guard;
        while condition(&mut *guard) {
            guard = self.wait(guard)?;
        }
        Ok(guard)
    }

    /// Waits on this condition variable for a notification, timing out after the specified duration (in milliseconds).
    pub fn wait_timeout_ms<'a, T>(&self, guard: MutexGuard<'a, T>, ms: u32) -> LockResult<(MutexGuard<'a, T>, bool)> {
        match self.wait_timeout(guard, Duration::from_millis(ms as u64)) {
            Ok((g, r)) => Ok((g, !r.timed_out())),
            Err(e) => {
                let (g, r) = e.into_inner();
                Err(PoisonError::new((g, !r.timed_out())))
            }
        }
    }

    /// Waits on this condition variable for a notification, timing out after a specified duration.
    pub fn wait_timeout<'a, T>(&self, guard: MutexGuard<'a, T>, dur: Duration) -> LockResult<(MutexGuard<'a, T>, WaitTimeoutResult)> {
        let lock = guard.lock;
        let woken = self.inner.wait(&lock.inner, duration_ns(dur));
        map_result(lock.poison.load(SeqCst), (guard, WaitTimeoutResult(!woken)))
    }

    /// Waits on this condition variable for a notification, timing out after a specified duration,
    /// while the condition holds.
    pub fn wait_timeout_while<'a, T, F>(
        &self,
        guard: MutexGuard<'a, T>,
        dur: Duration,
        condition: F,
    ) -> LockResult<(MutexGuard<'a, T>, WaitTimeoutResult)>
    where
        F: FnMut(&mut T) -> bool,
    {
        let mut condition = condition;
        let mut guard = guard;
        let start = Instant::now();
        loop {
            if !condition(&mut *guard) {
                return Ok((guard, WaitTimeoutResult(false)));
            }
            let timeout = match dur.checked_sub(start.elapsed()) {
                Some(timeout) => timeout,
                None => return Ok((guard, WaitTimeoutResult(true))),
            };
            match self.wait_timeout(guard, timeout) {
                Ok((g, _)) => guard = g,
                Err(e) => return Err(e),
            }
        }
    }

    /// Wakes up one blocked thread on this condvar.
    #[inline]
    pub fn notify_one(&self) {
        self.inner.notify_one()
    }

    /// Wakes up all blocked threads on this condvar.
    #[inline]
    pub fn notify_all(&self) {
        self.inner.notify_all()
    }
}

impl Default for Condvar {
    fn default() -> Condvar {
        Condvar::new()
    }
}

impl fmt::Debug for Condvar {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Condvar").finish_non_exhaustive()
    }
}
