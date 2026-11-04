//! One-time initialization: `Once`, `OnceLock` and `LazyLock`.

use super::sys::{futex_wait, futex_wake_all};
use crate::cell::UnsafeCell;
use crate::fmt;
use crate::ops::Deref;
use crate::sync::atomic::{AtomicU32, Ordering::SeqCst};

const INCOMPLETE: u32 = 0;
const POISONED: u32 = 1;
const RUNNING: u32 = 2;
const COMPLETE: u32 = 3;

/// A synchronization primitive which can be used to run a one-time global initialization.
pub struct Once {
    state: AtomicU32,
}

/// State yielded to `Once::call_once_force()`'s closure parameter.
pub struct OnceState {
    poisoned: bool,
    set_state_to: UnsafeCell<u32>,
}

impl OnceState {
    /// Returns `true` if the associated `Once` was poisoned prior to the invocation of the closure.
    #[inline]
    pub fn is_poisoned(&self) -> bool {
        self.poisoned
    }
}

impl fmt::Debug for OnceState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OnceState").field("poisoned", &self.is_poisoned()).finish()
    }
}

/// Initialization value for static `Once` values.
pub const ONCE_INIT: Once = Once::new();

// Sets the state when dropped (also on unwinding: then the `Once` is poisoned).
struct CompletionGuard<'a> {
    state: &'a AtomicU32,
    set_state_on_drop_to: u32,
}

impl<'a> Drop for CompletionGuard<'a> {
    fn drop(&mut self) {
        self.state.store(self.set_state_on_drop_to, SeqCst);
        futex_wake_all(self.state);
    }
}

impl Once {
    /// Creates a new `Once` value.
    #[inline]
    pub const fn new() -> Once {
        Once { state: AtomicU32::new(INCOMPLETE) }
    }

    /// Performs an initialization routine once and only once.
    #[track_caller]
    pub fn call_once<F: FnOnce()>(&self, f: F) {
        if self.state.load(SeqCst) == COMPLETE {
            return;
        }
        let mut f = Some(f);
        self.call(false, &mut |_| (f.take().unwrap())());
    }

    /// Performs the same function as `call_once()` except ignores poisoning.
    #[track_caller]
    pub fn call_once_force<F: FnOnce(&OnceState)>(&self, f: F) {
        if self.state.load(SeqCst) == COMPLETE {
            return;
        }
        let mut f = Some(f);
        self.call(true, &mut |s| (f.take().unwrap())(s));
    }

    #[track_caller]
    fn call(&self, ignore_poisoning: bool, f: &mut dyn FnMut(&OnceState)) {
        let mut state = self.state.load(SeqCst);
        loop {
            if state == POISONED && !ignore_poisoning {
                crate::rt::panic_str("Once instance has previously been poisoned", file!(), line!(), column!());
            }
            if state == INCOMPLETE || state == POISONED {
                if let Err(new) = self.state.compare_exchange_weak(state, RUNNING, SeqCst, SeqCst) {
                    state = new;
                    continue;
                }
                let mut guard = CompletionGuard { state: &self.state, set_state_on_drop_to: POISONED };
                let once_state = OnceState { poisoned: state == POISONED, set_state_to: UnsafeCell::new(COMPLETE) };
                f(&once_state);
                guard.set_state_on_drop_to = unsafe { *once_state.set_state_to.get() };
                return;
            }
            if state == RUNNING {
                futex_wait(&self.state, RUNNING, u64::MAX);
                state = self.state.load(SeqCst);
                continue;
            }
            return;
        }
    }

    /// Returns `true` if some `call_once()` call has completed successfully.
    #[inline]
    pub fn is_completed(&self) -> bool {
        self.state.load(SeqCst) == COMPLETE
    }

    /// Blocks the current thread until initialization has completed.
    #[track_caller]
    pub fn wait(&self) {
        loop {
            let s = self.state.load(SeqCst);
            if s == COMPLETE {
                return;
            }
            if s == POISONED {
                crate::rt::panic_str("Once instance has previously been poisoned", file!(), line!(), column!());
            }
            futex_wait(&self.state, s, u64::MAX);
        }
    }

    /// Blocks the current thread until initialization has completed, ignoring poisoning.
    pub fn wait_force(&self) {
        loop {
            let s = self.state.load(SeqCst);
            if s == COMPLETE || s == POISONED {
                return;
            }
            futex_wait(&self.state, s, u64::MAX);
        }
    }

    #[inline]
    fn is_poisoned(&self) -> bool {
        self.state.load(SeqCst) == POISONED
    }
}

impl fmt::Debug for Once {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Once").finish_non_exhaustive()
    }
}

// ---------------------------------------------------------------------------
// OnceLock

/// A synchronization primitive which can nominally be written to only once.
pub struct OnceLock<T> {
    once: Once,
    value: UnsafeCell<Option<T>>,
}

impl<T> OnceLock<T> {
    /// Creates a new uninitialized cell.
    #[inline]
    pub const fn new() -> OnceLock<T> {
        OnceLock { once: Once::new(), value: UnsafeCell::new(None) }
    }

    /// Gets the reference to the underlying value.
    #[inline]
    pub fn get(&self) -> Option<&T> {
        if self.once.is_completed() {
            unsafe { (*self.value.get()).as_ref() }
        } else {
            None
        }
    }

    /// Gets the mutable reference to the underlying value.
    #[inline]
    pub fn get_mut(&mut self) -> Option<&mut T> {
        self.value.get_mut().as_mut()
    }

    /// Blocks the current thread until the cell is initialized.
    pub fn wait(&self) -> &T {
        self.once.wait_force();
        self.get().unwrap()
    }

    /// Initializes the contents of the cell to `value`.
    pub fn set(&self, value: T) -> Result<(), T> {
        match self.try_insert(value) {
            Ok(_) => Ok(()),
            Err((_, value)) => Err(value),
        }
    }

    /// Initializes the contents of the cell to `value` if the cell was uninitialized, then returns a reference to it.
    pub fn try_insert(&self, value: T) -> Result<&T, (&T, T)> {
        let mut value = Some(value);
        let res = self.get_or_init(|| value.take().unwrap());
        match value {
            None => Ok(res),
            Some(value) => Err((res, value)),
        }
    }

    /// Gets the contents of the cell, initializing it to `f()` if the cell was uninitialized.
    pub fn get_or_init<F: FnOnce() -> T>(&self, f: F) -> &T {
        if let Some(v) = self.get() {
            return v;
        }
        let slot = self.value.get();
        let mut f = Some(f);
        self.once.call_once_force(|_| {
            let v = (f.take().unwrap())();
            unsafe {
                *slot = Some(v);
            }
        });
        self.get().unwrap()
    }

    /// Gets the contents of the cell, initializing it to `f()` if the cell was uninitialized; `f` may fail.
    pub fn get_or_try_init<F, E>(&self, f: F) -> Result<&T, E>
    where
        F: FnOnce() -> Result<T, E>,
    {
        if let Some(v) = self.get() {
            return Ok(v);
        }
        let slot = self.value.get();
        let mut res: Result<(), E> = Ok(());
        let mut f = Some(f);
        self.once.call_once_force(|s| match (f.take().unwrap())() {
            Ok(v) => unsafe {
                *slot = Some(v);
            },
            Err(e) => {
                res = Err(e);
                unsafe {
                    *s.set_state_to.get() = INCOMPLETE;
                }
            }
        });
        match res {
            Ok(()) => Ok(self.get().unwrap()),
            Err(e) => Err(e),
        }
    }

    /// Consumes the `OnceLock`, returning the wrapped value.
    pub fn into_inner(self) -> Option<T> {
        self.value.into_inner()
    }

    /// Takes the value out of this `OnceLock`, moving it back to an uninitialized state.
    pub fn take(&mut self) -> Option<T> {
        if self.once.is_completed() {
            self.once = Once::new();
            self.value.get_mut().take()
        } else {
            None
        }
    }
}

impl<T> Default for OnceLock<T> {
    fn default() -> OnceLock<T> {
        OnceLock::new()
    }
}

impl<T> From<T> for OnceLock<T> {
    fn from(value: T) -> Self {
        let cell = OnceLock::new();
        let _ = cell.set(value);
        cell
    }
}

impl<T: Clone> Clone for OnceLock<T> {
    fn clone(&self) -> OnceLock<T> {
        let cell = OnceLock::new();
        if let Some(value) = self.get() {
            let _ = cell.set(value.clone());
        }
        cell
    }
}

impl<T: PartialEq> PartialEq for OnceLock<T> {
    fn eq(&self, other: &OnceLock<T>) -> bool {
        self.get() == other.get()
    }
}

impl<T: Eq> Eq for OnceLock<T> {}

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

// ---------------------------------------------------------------------------
// LazyLock

/// A value which is initialized on the first access.
pub struct LazyLock<T, F = fn() -> T> {
    once: Once,
    init: UnsafeCell<Option<F>>,
    value: UnsafeCell<Option<T>>,
}

impl<T, F: FnOnce() -> T> LazyLock<T, F> {
    /// Creates a new lazy value with the given initializing function.
    #[inline]
    pub const fn new(f: F) -> LazyLock<T, F> {
        LazyLock { once: Once::new(), init: UnsafeCell::new(Some(f)), value: UnsafeCell::new(None) }
    }

    /// Consumes this `LazyLock` returning the stored value (or the initializer if never forced).
    pub fn into_inner(this: Self) -> Result<T, F> {
        let LazyLock { once: _, init, value } = this;
        match value.into_inner() {
            Some(v) => Ok(v),
            None => Err(init.into_inner().unwrap()),
        }
    }

    /// Forces the evaluation of this lazy value and returns a reference to result.
    #[track_caller]
    pub fn force(this: &LazyLock<T, F>) -> &T {
        if !this.once.is_completed() {
            if this.once.is_poisoned() {
                crate::rt::panic_str("LazyLock instance has previously been poisoned", file!(), line!(), column!());
            }
            let init = this.init.get();
            let slot = this.value.get();
            this.once.call_once(|| unsafe {
                let f = (*init).take().unwrap();
                *slot = Some(f());
            });
        }
        unsafe { (*this.value.get()).as_ref().unwrap() }
    }

    /// Forces the evaluation of this lazy value and returns a mutable reference to the result.
    #[track_caller]
    pub fn force_mut(this: &mut LazyLock<T, F>) -> &mut T {
        LazyLock::force(this);
        unsafe { (*this.value.get()).as_mut().unwrap() }
    }
}

impl<T, F> LazyLock<T, F> {
    /// Returns a reference to the value if initialized, or `None` if not.
    pub fn get(this: &LazyLock<T, F>) -> Option<&T> {
        if this.once.is_completed() {
            unsafe { (*this.value.get()).as_ref() }
        } else {
            None
        }
    }
}

impl<T, F: FnOnce() -> T> Deref for LazyLock<T, F> {
    type Target = T;
    #[inline]
    fn deref(&self) -> &T {
        LazyLock::force(self)
    }
}

impl<T: Default> Default for LazyLock<T> {
    fn default() -> LazyLock<T> {
        LazyLock::new(T::default)
    }
}

impl<T: fmt::Debug, F> fmt::Debug for LazyLock<T, F> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut d = f.debug_tuple("LazyLock");
        match LazyLock::get(self) {
            Some(v) => d.field(v),
            None => d.field(&format_args!("<uninit>")),
        };
        d.finish()
    }
}
