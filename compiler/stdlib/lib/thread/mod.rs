//! Native threads.

mod local;
mod scoped;

pub use self::local::{AccessError, LocalKey};
pub use self::scoped::{scope, Scope, ScopedJoinHandle};

use crate::any::Any;
use crate::cell::{RefCell, UnsafeCell};
use crate::fmt;
use crate::intrinsics;
use crate::num::NonZeroUsize;
use crate::sync::atomic::{AtomicU32, AtomicU64, Ordering::SeqCst};
use crate::sync::sys::{futex_wait, futex_wake};
use crate::sync::Arc;
use crate::time::{Duration, Instant};

/// A specialized `Result` type for threads.
pub type Result<T> = crate::result::Result<T, Box<dyn Any + Send + 'static>>;

// ---------------------------------------------------------------------------
// ThreadId / Thread

/// A unique identifier for a running thread.
#[derive(Eq, PartialEq, Clone, Copy, Hash, Debug, PartialOrd, Ord)]
pub struct ThreadId(u64);

// The main thread is ThreadId(1); spawned threads are numbered from 2.
static NEXT_ID: AtomicU64 = AtomicU64::new(2);

impl ThreadId {
    fn new() -> ThreadId {
        ThreadId(NEXT_ID.fetch_add(1, SeqCst))
    }

    /// This returns a numeric identifier for the thread identified by this `ThreadId`.
    pub fn as_u64(&self) -> crate::num::NonZeroU64 {
        crate::num::NonZeroU64::new(self.0).unwrap()
    }
}

const PARK_EMPTY: u32 = 0;
const PARK_NOTIFIED: u32 = 1;
const PARK_PARKED: u32 = u32::MAX;

struct Inner {
    name: Option<String>,
    id: ThreadId,
    park_state: AtomicU32,
}

/// A handle to a thread.
#[derive(Clone)]
pub struct Thread {
    inner: Arc<Inner>,
}

impl Thread {
    fn new(id: ThreadId, name: Option<String>) -> Thread {
        Thread { inner: Arc::new(Inner { name, id, park_state: AtomicU32::new(PARK_EMPTY) }) }
    }

    /// Gets the thread's unique identifier.
    #[inline]
    pub fn id(&self) -> ThreadId {
        self.inner.id
    }

    /// Gets the thread's name.
    pub fn name(&self) -> Option<&str> {
        match &self.inner.name {
            Some(s) => Some(s.as_str()),
            None => None,
        }
    }

    /// Atomically makes the handle's token available if it is not already.
    pub fn unpark(&self) {
        if self.inner.park_state.swap(PARK_NOTIFIED, SeqCst) == PARK_PARKED {
            futex_wake(&self.inner.park_state);
        }
    }

    fn park(&self) {
        let state = &self.inner.park_state;
        if state.fetch_sub(1, SeqCst) == PARK_NOTIFIED {
            return;
        }
        loop {
            futex_wait(state, PARK_PARKED, u64::MAX);
            if state.compare_exchange(PARK_NOTIFIED, PARK_EMPTY, SeqCst, SeqCst).is_ok() {
                return;
            }
        }
    }

    fn park_timeout(&self, dur: Duration) {
        let state = &self.inner.park_state;
        if state.fetch_sub(1, SeqCst) == PARK_NOTIFIED {
            return;
        }
        futex_wait(state, PARK_PARKED, dur.as_nanos_u64_saturating());
        if state.swap(PARK_EMPTY, SeqCst) == PARK_NOTIFIED {
            // received a notification
        }
    }
}

impl fmt::Debug for Thread {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Thread").field("id", &self.id()).field("name", &self.name()).finish_non_exhaustive()
    }
}

thread_local! {
    static CURRENT: RefCell<Option<Thread>> = RefCell::new(None);
}

fn set_current(thread: Thread) {
    CURRENT.with(|c| *c.borrow_mut() = Some(thread));
}

/// Gets a handle to the thread that invokes it.
pub fn current() -> Thread {
    CURRENT.with(|c| {
        if let Some(t) = &*c.borrow() {
            return t.clone();
        }
        let t = if unsafe { intrinsics::thread_is_main() } {
            Thread::new(ThreadId(1), Some(String::from("main")))
        } else {
            Thread::new(ThreadId::new(), None)
        };
        *c.borrow_mut() = Some(t.clone());
        t
    })
}

/// Blocks unless or until the current thread's token is made available.
pub fn park() {
    current().park();
}

/// Blocks unless or until the current thread's token is made available or the specified duration has been reached.
pub fn park_timeout(dur: Duration) {
    current().park_timeout(dur);
}

/// Use `park_timeout` (deprecated).
pub fn park_timeout_ms(ms: u32) {
    park_timeout(Duration::from_millis(ms as u64));
}

/// Puts the current thread to sleep for at least the specified amount of time.
pub fn sleep(dur: Duration) {
    unsafe { intrinsics::thread_sleep_ns(dur.as_nanos_u64_saturating()) }
}

/// Puts the current thread to sleep for at least the specified amount of milliseconds (deprecated).
pub fn sleep_ms(ms: u32) {
    sleep(Duration::from_millis(ms as u64))
}

/// Puts the current thread to sleep until the specified deadline has passed.
pub fn sleep_until(deadline: Instant) {
    let now = Instant::now();
    if deadline > now {
        sleep(deadline - now);
    }
}

/// Cooperatively gives up a timeslice to the OS scheduler.
pub fn yield_now() {
    unsafe { intrinsics::thread_yield() }
}

/// Determines whether the current thread is unwinding because of panic.
#[inline]
pub fn panicking() -> bool {
    unsafe { intrinsics::thread_panicking() }
}

/// Returns an estimate of the default amount of parallelism a program should use.
pub fn available_parallelism() -> crate::io::Result<NonZeroUsize> {
    let n = unsafe { intrinsics::available_parallelism() };
    Ok(NonZeroUsize::new(if n == 0 { 1 } else { n }).unwrap())
}

// ---------------------------------------------------------------------------
// spawning

// The result slot shared by a thread and its join handle.
pub(crate) struct Packet<T> {
    result: UnsafeCell<Option<Result<T>>>,
    scope: Option<Arc<scoped::ScopeData>>,
}

impl<T> Drop for Packet<T> {
    fn drop(&mut self) {
        let unhandled_panic = match self.result.get_mut() {
            Some(Err(_)) => true,
            _ => false,
        };
        // Drop the result before signalling the scope (it may borrow from it).
        *self.result.get_mut() = None;
        if let Some(scope) = &self.scope {
            scope.decrement_num_running_threads(unhandled_panic);
        }
    }
}

// An owned reference to a native thread.
struct Native(u64);

impl Drop for Native {
    fn drop(&mut self) {
        unsafe { intrinsics::thread_release(self.0) }
    }
}

pub(crate) struct JoinInner<T> {
    native: Native,
    thread: Thread,
    packet: Arc<Packet<T>>,
}

impl<T> JoinInner<T> {
    fn join(self) -> Result<T> {
        unsafe {
            intrinsics::thread_join(self.native.0);
            (*self.packet.result.get()).take().unwrap()
        }
    }

    fn is_finished(&self) -> bool {
        unsafe { intrinsics::thread_is_finished(self.native.0) }
    }
}

struct SpawnData<F, T> {
    f: F,
    packet: Arc<Packet<T>>,
    thread: Thread,
}

fn thread_main<F, T>(data: *mut u8)
where
    F: FnOnce() -> T,
{
    let data = unsafe { Box::from_raw(data as *mut SpawnData<F, T>) };
    let SpawnData { f, packet, thread } = Box::into_inner(data);
    if let Some(name) = thread.name() {
        unsafe { intrinsics::thread_set_name(name.as_ptr(), name.len()) };
    }
    set_current(thread);
    let r = crate::panicking::catch(f);
    unsafe {
        *packet.result.get() = Some(r);
    }
    drop(packet);
}

// Spawns a thread (the closure's lifetime is not checked).
pub(crate) fn spawn_unchecked<F, T>(
    name: Option<String>,
    stack_size: usize,
    f: F,
    scope: Option<Arc<scoped::ScopeData>>,
) -> crate::io::Result<JoinInner<T>>
where
    F: FnOnce() -> T,
{
    let thread = Thread::new(ThreadId::new(), name);
    let packet: Arc<Packet<T>> = Arc::new(Packet { result: UnsafeCell::new(None), scope: scope.clone() });
    if let Some(s) = &scope {
        s.increment_num_running_threads();
    }
    let data = Box::new(SpawnData { f, packet: packet.clone(), thread: thread.clone() });
    let raw = Box::into_raw(data);
    let h = unsafe { intrinsics::thread_spawn(thread_main::<F, T>, raw as *mut u8, stack_size) };
    if h == 0 {
        unsafe {
            drop(Box::from_raw(raw));
        }
        return Err(crate::io::Error::new(crate::io::ErrorKind::Other, "failed to spawn thread"));
    }
    Ok(JoinInner { native: Native(h), thread, packet })
}

/// Spawns a new thread, returning a `JoinHandle` for it.
#[track_caller]
pub fn spawn<F, T>(f: F) -> JoinHandle<T>
where
    F: FnOnce() -> T + Send + 'static,
    T: Send + 'static,
{
    match Builder::new().spawn(f) {
        Ok(h) => h,
        Err(e) => panic!("failed to spawn thread: {:?}", e),
    }
}

/// Thread factory, which can be used in order to configure the properties of a new thread.
#[derive(Debug)]
pub struct Builder {
    name: Option<String>,
    stack_size: Option<usize>,
    no_hooks: bool,
}

impl Builder {
    /// Generates the base configuration for spawning a thread.
    pub fn new() -> Builder {
        Builder { name: None, stack_size: None, no_hooks: false }
    }

    /// Names the thread-to-be.
    pub fn name(self, name: String) -> Builder {
        Builder { name: Some(name), stack_size: self.stack_size, no_hooks: self.no_hooks }
    }

    /// Sets the size of the stack (in bytes) for the new thread.
    pub fn stack_size(self, size: usize) -> Builder {
        Builder { name: self.name, stack_size: Some(size), no_hooks: self.no_hooks }
    }

    /// Spawns a new thread by taking ownership of the `Builder`, and returns an `io::Result` to its `JoinHandle`.
    pub fn spawn<F, T>(self, f: F) -> crate::io::Result<JoinHandle<T>>
    where
        F: FnOnce() -> T + Send + 'static,
        T: Send + 'static,
    {
        unsafe { self.spawn_unchecked(f) }
    }

    /// Spawns a new thread without any lifetime restrictions.
    pub unsafe fn spawn_unchecked<F, T>(self, f: F) -> crate::io::Result<JoinHandle<T>>
    where
        F: FnOnce() -> T + Send,
        T: Send,
    {
        let stack = match self.stack_size {
            Some(s) => s,
            None => 0,
        };
        match spawn_unchecked(self.name, stack, f, None) {
            Ok(inner) => Ok(JoinHandle(inner)),
            Err(e) => Err(e),
        }
    }

    /// Spawns a new scoped thread using the settings set through this `Builder`.
    pub fn spawn_scoped<'scope, 'env, F, T>(self, scope: &'scope Scope<'scope, 'env>, f: F) -> crate::io::Result<ScopedJoinHandle<'scope, T>>
    where
        F: FnOnce() -> T + Send + 'scope,
        T: Send + 'scope,
    {
        let stack = match self.stack_size {
            Some(s) => s,
            None => 0,
        };
        scope.spawn_with(self.name, stack, f)
    }
}

/// An owned permission to join on a thread (block on its termination).
pub struct JoinHandle<T>(JoinInner<T>);

impl<T> JoinHandle<T> {
    /// Extracts a handle to the underlying thread.
    pub fn thread(&self) -> &Thread {
        &self.0.thread
    }

    /// Waits for the associated thread to finish.
    pub fn join(self) -> Result<T> {
        self.0.join()
    }

    /// Checks if the associated thread has finished running its main function.
    pub fn is_finished(&self) -> bool {
        self.0.is_finished()
    }
}

impl<T> fmt::Debug for JoinHandle<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("JoinHandle").finish_non_exhaustive()
    }
}

impl<T> crate::os::unix::thread::JoinHandleExt for JoinHandle<T> {
    fn as_pthread_t(&self) -> crate::os::unix::thread::RawPthread {
        (self.0).native.0
    }

    fn into_pthread_t(self) -> crate::os::unix::thread::RawPthread {
        let h = (self.0).native.0;
        crate::mem::forget(self);
        h
    }
}
