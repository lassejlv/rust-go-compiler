//! Barriers.

use super::condvar::Condvar;
use super::mutex::Mutex;
use crate::fmt;

struct BarrierState {
    count: usize,
    generation_id: usize,
}

/// A barrier enables multiple threads to synchronize the beginning of some computation.
pub struct Barrier {
    lock: Mutex<BarrierState>,
    cvar: Condvar,
    num_threads: usize,
}

/// A `BarrierWaitResult` is returned by `Barrier::wait()` when all threads in the `Barrier` have rendezvoused.
pub struct BarrierWaitResult(bool);

impl BarrierWaitResult {
    /// Returns `true` if this thread is the "leader thread" for the call to `Barrier::wait()`.
    #[inline]
    pub fn is_leader(&self) -> bool {
        self.0
    }
}

impl fmt::Debug for BarrierWaitResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BarrierWaitResult").field("is_leader", &self.is_leader()).finish()
    }
}

impl Barrier {
    /// Creates a new barrier that can block a given number of threads.
    pub const fn new(n: usize) -> Barrier {
        Barrier { lock: Mutex::new(BarrierState { count: 0, generation_id: 0 }), cvar: Condvar::new(), num_threads: n }
    }

    /// Blocks the current thread until all threads have rendezvoused here.
    pub fn wait(&self) -> BarrierWaitResult {
        let mut lock = self.lock.lock().unwrap();
        let local_gen = lock.generation_id;
        lock.count += 1;
        if lock.count < self.num_threads {
            let _guard = self.cvar.wait_while(lock, |state| local_gen == state.generation_id).unwrap();
            BarrierWaitResult(false)
        } else {
            lock.count = 0;
            lock.generation_id = lock.generation_id.wrapping_add(1);
            self.cvar.notify_all();
            BarrierWaitResult(true)
        }
    }
}

impl fmt::Debug for Barrier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Barrier").finish_non_exhaustive()
    }
}
