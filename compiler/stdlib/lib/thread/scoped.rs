//! Scoped threads.

use super::{spawn_unchecked, JoinInner, Result, Thread};
use crate::fmt;
use crate::marker::PhantomData;
use crate::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering::SeqCst};
use crate::sync::sys::{futex_wait, futex_wake_all};
use crate::sync::Arc;

pub(crate) struct ScopeData {
    main_thread: Thread,
    num_running_threads: AtomicUsize,
    a_thread_panicked: AtomicBool,
    seq: AtomicU32,
}

impl ScopeData {
    pub(crate) fn increment_num_running_threads(&self) {
        self.num_running_threads.fetch_add(1, SeqCst);
    }

    pub(crate) fn decrement_num_running_threads(&self, panic: bool) {
        if panic {
            self.a_thread_panicked.store(true, SeqCst);
        }
        if self.num_running_threads.fetch_sub(1, SeqCst) == 1 {
            self.seq.fetch_add(1, SeqCst);
            futex_wake_all(&self.seq);
        }
    }

    fn wait_all(&self) {
        loop {
            let v = self.seq.load(SeqCst);
            if self.num_running_threads.load(SeqCst) == 0 {
                return;
            }
            futex_wait(&self.seq, v, u64::MAX);
        }
    }
}

/// A scope to spawn scoped threads in.
pub struct Scope<'scope, 'env: 'scope> {
    data: Arc<ScopeData>,
    scope: PhantomData<&'scope mut &'scope ()>,
    env: PhantomData<&'env mut &'env ()>,
}

/// An owned permission to join on a scoped thread (block on its termination).
pub struct ScopedJoinHandle<'scope, T> {
    inner: JoinInner<T>,
    _marker: PhantomData<&'scope ()>,
}

// Waits for the scope's threads when dropped (also while unwinding).
struct WaitGuard {
    data: Arc<ScopeData>,
}

impl Drop for WaitGuard {
    fn drop(&mut self) {
        self.data.wait_all();
    }
}

/// Create a scope for spawning scoped threads.
#[track_caller]
pub fn scope<'env, F, T>(f: F) -> T
where
    F: for<'scope> FnOnce(&'scope Scope<'scope, 'env>) -> T,
{
    let data = Arc::new(ScopeData {
        main_thread: super::current(),
        num_running_threads: AtomicUsize::new(0),
        a_thread_panicked: AtomicBool::new(false),
        seq: AtomicU32::new(0),
    });
    let scope = Scope { data: data.clone(), scope: PhantomData, env: PhantomData };
    let guard = WaitGuard { data: data.clone() };
    let result = f(&scope);
    drop(guard);
    if data.a_thread_panicked.load(SeqCst) {
        panic!("a scoped thread panicked");
    }
    result
}

impl<'scope, 'env> Scope<'scope, 'env> {
    /// Spawns a new thread within a scope, returning a `ScopedJoinHandle` for it.
    #[track_caller]
    pub fn spawn<F, T>(&'scope self, f: F) -> ScopedJoinHandle<'scope, T>
    where
        F: FnOnce() -> T + Send + 'scope,
        T: Send + 'scope,
    {
        match self.spawn_with(None, 0, f) {
            Ok(h) => h,
            Err(e) => panic!("failed to spawn thread: {:?}", e),
        }
    }

    pub(crate) fn spawn_with<F, T>(&'scope self, name: Option<String>, stack: usize, f: F) -> crate::io::Result<ScopedJoinHandle<'scope, T>>
    where
        F: FnOnce() -> T + Send + 'scope,
        T: Send + 'scope,
    {
        match spawn_unchecked(name, stack, f, Some(self.data.clone())) {
            Ok(inner) => Ok(ScopedJoinHandle { inner, _marker: PhantomData }),
            Err(e) => Err(e),
        }
    }
}

impl<'scope, T> ScopedJoinHandle<'scope, T> {
    /// Extracts a handle to the underlying thread.
    pub fn thread(&self) -> &Thread {
        &self.inner.thread
    }

    /// Waits for the associated thread to finish.
    pub fn join(self) -> Result<T> {
        self.inner.join()
    }

    /// Checks if the associated thread has finished running its main function.
    pub fn is_finished(&self) -> bool {
        self.inner.is_finished()
    }
}

impl<'scope, 'env> fmt::Debug for Scope<'scope, 'env> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Scope")
            .field("num_running_threads", &self.data.num_running_threads.load(SeqCst))
            .field("a_thread_panicked", &self.data.a_thread_panicked.load(SeqCst))
            .field("main_thread", &self.data.main_thread)
            .finish_non_exhaustive()
    }
}

impl<'scope, T> fmt::Debug for ScopedJoinHandle<'scope, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ScopedJoinHandle").finish_non_exhaustive()
    }
}
