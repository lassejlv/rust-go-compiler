//! Asynchronous values.

use crate::boxed::Box;
use crate::fmt;
use crate::marker::Unpin;
use crate::ops::DerefMut;
use crate::pin::Pin;
use crate::task::{Context, Poll};

mod coro {
    extern "rust-intrinsic" {
        pub fn coro_new(entry: fn(*mut u8), data: *mut u8) -> *mut u8;
        pub fn coro_resume(co: *mut u8) -> bool;
        pub fn coro_yield();
        pub fn coro_free(co: *mut u8);
        pub fn coro_get_cx() -> *mut u8;
        pub fn coro_set_cx(cx: *mut u8);
    }
}

/// An asynchronous computation.
#[lang = "future_trait"]
pub trait Future {
    type Output;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output>;
}

/// Conversion into a `Future`.
pub trait IntoFuture {
    type Output;
    type IntoFuture: Future<Output = Self::Output>;
    fn into_future(self) -> Self::IntoFuture;
}

impl<F: Future> IntoFuture for F {
    type Output = F::Output;
    type IntoFuture = F;
    fn into_future(self) -> F {
        self
    }
}

impl<F: Future + ?Sized + Unpin> Future for &mut F {
    type Output = F::Output;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<F::Output> {
        let inner: &mut F = unsafe { &mut **self.get_unchecked_mut() };
        unsafe { Pin::new_unchecked(inner) }.poll(cx)
    }
}

impl<F: Future + ?Sized + Unpin> Future for Box<F> {
    type Output = F::Output;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<F::Output> {
        let inner: &mut F = unsafe { &mut **self.get_unchecked_mut() };
        unsafe { Pin::new_unchecked(inner) }.poll(cx)
    }
}

impl<P> Future for Pin<P>
where
    P: DerefMut,
    P::Target: Future,
{
    type Output = <<P as crate::ops::Deref>::Target as Future>::Output;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        unsafe { self.get_unchecked_mut().as_mut() }.poll(cx)
    }
}

/// A future that is immediately ready with a value.
#[derive(Debug, Clone)]
pub struct Ready<T>(Option<T>);

impl<T> Unpin for Ready<T> {}

impl<T> Future for Ready<T> {
    type Output = T;
    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<T> {
        let this = unsafe { self.get_unchecked_mut() };
        Poll::Ready(this.0.take().expect("`Ready` polled after completion"))
    }
}

impl<T> Ready<T> {
    pub fn into_inner(self) -> T {
        self.0.expect("Called `into_inner()` on `Ready` after completion")
    }
}

/// Creates a future that is immediately ready with a value.
pub fn ready<T>(t: T) -> Ready<T> {
    Ready(Some(t))
}

/// A future which never resolves.
#[derive(Debug)]
pub struct Pending<T> {
    _p: crate::marker::PhantomData<T>,
}

pub fn pending<T>() -> Pending<T> {
    Pending { _p: crate::marker::PhantomData }
}

impl<T> Future for Pending<T> {
    type Output = T;
    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<T> {
        Poll::Pending
    }
}

/// A future wrapping a polling closure.
pub struct PollFn<F> {
    f: F,
}

impl<F> Unpin for PollFn<F> {}

pub fn poll_fn<T, F: FnMut(&mut Context<'_>) -> Poll<T>>(f: F) -> PollFn<F> {
    PollFn { f }
}

impl<T, F: FnMut(&mut Context<'_>) -> Poll<T>> Future for PollFn<F> {
    type Output = T;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<T> {
        let this = unsafe { self.get_unchecked_mut() };
        (this.f)(cx)
    }
}

impl<F> fmt::Debug for PollFn<F> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("PollFn")
    }
}

// ---------------------------------------------------------------------------
// Compiler support for `async` blocks and functions.
//
// `async { body }` is lowered to `__async_block(|| body)`. The body runs on
// its own coroutine stack; `.await` polls the awaited future and, while it
// is pending, yields back to whoever polled the async block.

#[doc(hidden)]
pub struct AsyncBlock<F, T> {
    f: Option<F>,
    result: Option<T>,
    co: *mut u8,
    done: bool,
}

#[doc(hidden)]
pub fn __async_block<F: FnOnce() -> T, T>(f: F) -> AsyncBlock<F, T> {
    AsyncBlock { f: Some(f), result: None, co: 0 as *mut u8, done: false }
}

fn async_trampoline<F: FnOnce() -> T, T>(p: *mut u8) {
    let this = unsafe { &mut *(p as *mut AsyncBlock<F, T>) };
    let f = this.f.take().unwrap();
    let r = f();
    this.result = Some(r);
}

impl<F: FnOnce() -> T, T> Future for AsyncBlock<F, T> {
    type Output = T;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<T> {
        let this = unsafe { self.get_unchecked_mut() };
        if this.done {
            panic!("`async fn` resumed after completion");
        }
        if this.co.is_null() {
            let entry: fn(*mut u8) = async_trampoline::<F, T>;
            this.co = unsafe { coro::coro_new(entry, this as *mut AsyncBlock<F, T> as *mut u8) };
        }
        let prev = unsafe { coro::coro_get_cx() };
        unsafe { coro::coro_set_cx(cx as *mut Context<'_> as *mut u8) };
        let finished = unsafe { coro::coro_resume(this.co) };
        unsafe { coro::coro_set_cx(prev) };
        if finished {
            unsafe { coro::coro_free(this.co) };
            this.co = 0 as *mut u8;
            this.done = true;
            Poll::Ready(this.result.take().unwrap())
        } else {
            Poll::Pending
        }
    }
}

impl<F, T> Drop for AsyncBlock<F, T> {
    fn drop(&mut self) {
        if !self.co.is_null() {
            unsafe { coro::coro_free(self.co) };
            self.co = 0 as *mut u8;
        }
    }
}

impl<F, T> fmt::Debug for AsyncBlock<F, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("{async block}")
    }
}

/// The `Context` of the innermost async block being polled.
#[doc(hidden)]
pub fn __current_cx() -> &'static mut Context<'static> {
    unsafe { &mut *(coro::coro_get_cx() as *mut Context<'static>) }
}

/// Suspends the current async block (called by `.await` on `Pending`).
#[doc(hidden)]
pub fn __yield_now() {
    unsafe { coro::coro_yield() }
}
