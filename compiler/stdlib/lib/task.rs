//! Types and traits for working with asynchronous tasks.

use crate::fmt;
use crate::mem;
use crate::sync::Arc;

/// Indicates whether a value is available.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[lang = "Poll"]
pub enum Poll<T> {
    #[lang = "Ready"]
    Ready(T),
    #[lang = "Pending"]
    Pending,
}

impl<T> Poll<T> {
    pub fn map<U, F: FnOnce(T) -> U>(self, f: F) -> Poll<U> {
        match self {
            Poll::Ready(t) => Poll::Ready(f(t)),
            Poll::Pending => Poll::Pending,
        }
    }
    pub fn is_ready(&self) -> bool {
        match self {
            Poll::Ready(_) => true,
            Poll::Pending => false,
        }
    }
    pub fn is_pending(&self) -> bool {
        !self.is_ready()
    }
}

impl<T> From<T> for Poll<T> {
    fn from(t: T) -> Poll<T> {
        Poll::Ready(t)
    }
}

/// A virtual function pointer table for a `RawWaker`.
#[derive(Copy, Clone, PartialEq)]
pub struct RawWakerVTable {
    clone: unsafe fn(*const ()) -> RawWaker,
    wake: unsafe fn(*const ()),
    wake_by_ref: unsafe fn(*const ()),
    drop: unsafe fn(*const ()),
}

impl RawWakerVTable {
    #[rustc_promotable]
    pub const fn new(
        clone: unsafe fn(*const ()) -> RawWaker,
        wake: unsafe fn(*const ()),
        wake_by_ref: unsafe fn(*const ()),
        drop: unsafe fn(*const ()),
    ) -> RawWakerVTable {
        RawWakerVTable { clone, wake, wake_by_ref, drop }
    }
}

impl fmt::Debug for RawWakerVTable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("RawWakerVTable")
    }
}

/// A raw waker: a data pointer and a vtable.
#[derive(PartialEq, Debug)]
pub struct RawWaker {
    data: *const (),
    vtable: &'static RawWakerVTable,
}

impl RawWaker {
    pub const fn new(data: *const (), vtable: &'static RawWakerVTable) -> RawWaker {
        RawWaker { data, vtable }
    }
    pub fn data(&self) -> *const () {
        self.data
    }
    pub fn vtable(&self) -> &'static RawWakerVTable {
        self.vtable
    }
}

/// A handle for waking up a task.
pub struct Waker {
    waker: RawWaker,
}

impl Waker {
    pub unsafe fn from_raw(waker: RawWaker) -> Waker {
        Waker { waker }
    }
    pub unsafe fn new(data: *const (), vtable: &'static RawWakerVTable) -> Waker {
        Waker { waker: RawWaker { data, vtable } }
    }
    pub fn wake(self) {
        let w = self.waker.vtable.wake;
        let d = self.waker.data;
        mem::forget(self);
        unsafe { w(d) }
    }
    pub fn wake_by_ref(&self) {
        unsafe { (self.waker.vtable.wake_by_ref)(self.waker.data) }
    }
    pub fn will_wake(&self, other: &Waker) -> bool {
        self.waker.data == other.waker.data && self.waker.vtable == other.waker.vtable
    }
    pub fn data(&self) -> *const () {
        self.waker.data
    }
    pub fn vtable(&self) -> &'static RawWakerVTable {
        self.waker.vtable
    }
    /// A waker that does nothing when woken.
    pub fn noop() -> &'static Waker {
        // (one static waker, like std's `const WAKER: &Waker`)
        &NOOP_WAKER.0
    }
}

struct NoopWaker(Waker);
// the noop waker's data is null and its functions do nothing
unsafe impl Sync for NoopWaker {}
static NOOP_WAKER: NoopWaker = NoopWaker(Waker { waker: RawWaker::new(0 as *const (), &NOOP_VTABLE) });
static NOOP_VTABLE: RawWakerVTable = RawWakerVTable::new(noop_clone, noop_fn, noop_fn, noop_fn);

unsafe fn noop_clone(_p: *const ()) -> RawWaker {
    Waker::noop().waker.clone_raw()
}
unsafe fn noop_fn(_p: *const ()) {}

impl RawWaker {
    fn clone_raw(&self) -> RawWaker {
        RawWaker { data: self.data, vtable: self.vtable }
    }
}

impl Clone for Waker {
    fn clone(&self) -> Waker {
        Waker { waker: unsafe { (self.waker.vtable.clone)(self.waker.data) } }
    }
}

impl Drop for Waker {
    fn drop(&mut self) {
        unsafe { (self.waker.vtable.drop)(self.waker.data) }
    }
}

impl fmt::Debug for Waker {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Waker").field("data", &self.waker.data).finish()
    }
}

/// The context of an asynchronous task.
pub struct Context<'a> {
    waker: &'a Waker,
}

impl<'a> Context<'a> {
    pub fn from_waker(waker: &'a Waker) -> Context<'a> {
        Context { waker }
    }
    pub fn waker(&self) -> &'a Waker {
        self.waker
    }
}

impl<'a> fmt::Debug for Context<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Context").field("waker", &self.waker).finish()
    }
}

/// The implementation of waking a task on an executor.
pub trait Wake {
    fn wake(self: Arc<Self>);
    fn wake_by_ref(self: &Arc<Self>) {
        self.clone().wake();
    }
}

unsafe fn arc_clone<W: Wake + Send + Sync + 'static>(p: *const ()) -> RawWaker {
    let a: Arc<W> = Arc::from_raw(p as *const W);
    let b = a.clone();
    mem::forget(a);
    RawWaker::new(Arc::into_raw(b) as *const (), arc_vtable::<W>())
}

unsafe fn arc_wake<W: Wake + Send + Sync + 'static>(p: *const ()) {
    let a: Arc<W> = Arc::from_raw(p as *const W);
    a.wake();
}

unsafe fn arc_wake_by_ref<W: Wake + Send + Sync + 'static>(p: *const ()) {
    let a: Arc<W> = Arc::from_raw(p as *const W);
    a.wake_by_ref();
    mem::forget(a);
}

unsafe fn arc_drop<W: Wake + Send + Sync + 'static>(p: *const ()) {
    let a: Arc<W> = Arc::from_raw(p as *const W);
    drop(a);
}

// (a promoted static per waker type: wakers of one type share their vtable,
// and creating or cloning one allocates nothing)
fn arc_vtable<W: Wake + Send + Sync + 'static>() -> &'static RawWakerVTable {
    &RawWakerVTable::new(arc_clone::<W>, arc_wake::<W>, arc_wake_by_ref::<W>, arc_drop::<W>)
}

impl<W: Wake + Send + Sync + 'static> From<Arc<W>> for Waker {
    fn from(waker: Arc<W>) -> Waker {
        Waker { waker: RawWaker::new(Arc::into_raw(waker) as *const (), arc_vtable::<W>()) }
    }
}

impl<W: Wake + Send + Sync + 'static> From<Arc<W>> for RawWaker {
    fn from(waker: Arc<W>) -> RawWaker {
        RawWaker::new(Arc::into_raw(waker) as *const (), arc_vtable::<W>())
    }
}
