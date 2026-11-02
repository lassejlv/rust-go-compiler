//! Thread-local storage (`thread_local!`).

use crate::cell::{Cell, RefCell};
use crate::fmt;
use crate::intrinsics;
use crate::sync::atomic::{AtomicUsize, Ordering::SeqCst};

/// A thread local storage key which owns its contents.
///
/// Each thread lazily initializes its own value (with the initializer given
/// to `thread_local!`) on the first access; the value is dropped when the
/// thread exits.
pub struct LocalKey<T: 'static> {
    key: AtomicUsize,
    init: fn() -> T,
}

/// An error returned by `LocalKey::try_with`.
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct AccessError {
    _private: (),
}

impl fmt::Debug for AccessError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AccessError").finish()
    }
}

impl fmt::Display for AccessError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("already destroyed")
    }
}

impl crate::error::Error for AccessError {}

impl<T: 'static> fmt::Debug for LocalKey<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LocalKey").finish_non_exhaustive()
    }
}

fn tls_dtor<T>(p: *mut u8) {
    unsafe {
        drop(Box::from_raw(p as *mut T));
    }
}

impl<T: 'static> LocalKey<T> {
    /// Creates a key whose per-thread values are created by `init` (used by `thread_local!`).
    #[doc(hidden)]
    pub const fn new(init: fn() -> T) -> LocalKey<T> {
        LocalKey { key: AtomicUsize::new(0), init }
    }

    fn key(&self) -> usize {
        let k = self.key.load(SeqCst);
        if k != 0 {
            return k;
        }
        let n = unsafe { intrinsics::tls_new_key() };
        match self.key.compare_exchange(0, n, SeqCst, SeqCst) {
            Ok(_) => n,
            Err(old) => old,
        }
    }

    // Returns this thread's value, initializing it with `value` (or the
    // initializer) first. Null after the value has been destroyed. The flag
    // reports whether `value` was used.
    fn slot(&'static self, value: Option<T>) -> (*const T, Option<T>) {
        unsafe {
            let k = self.key();
            let p = intrinsics::tls_get(k);
            if (p as usize) == 1 {
                return (0usize as *const T, value);
            }
            if !(p as usize == 0) {
                return (p as *const T, value);
            }
            let v = match value {
                Some(v) => v,
                None => (self.init)(),
            };
            let old = intrinsics::tls_get(k);
            let b = Box::into_raw(Box::new(v));
            intrinsics::tls_set(k, b as *mut u8, tls_dtor::<T>);
            if !(old as usize == 0) && (old as usize) != 1 {
                // the initializer recursively initialized this key
                drop(Box::from_raw(old as *mut T));
            }
            (b as *const T, None)
        }
    }

    /// Acquires a reference to the value in this TLS key.
    #[track_caller]
    pub fn with<F, R>(&'static self, f: F) -> R
    where
        F: FnOnce(&T) -> R,
    {
        let (p, _) = self.slot(None);
        if (p as usize) == 0 {
            crate::rt::panic_str(
                "cannot access a Thread Local Storage value during or after destruction: AccessError",
                file!(),
                line!(),
                column!(),
            );
        }
        f(unsafe { &*p })
    }

    /// Acquires a reference to the value in this TLS key, or returns an error if it has been destroyed.
    pub fn try_with<F, R>(&'static self, f: F) -> Result<R, AccessError>
    where
        F: FnOnce(&T) -> R,
    {
        let (p, _) = self.slot(None);
        if (p as usize) == 0 {
            return Err(AccessError { _private: () });
        }
        Ok(f(unsafe { &*p }))
    }

    // Initializes the value with `init` if uninitialized (f gets None), else
    // calls f with `Some(init)` and the existing value.
    #[track_caller]
    fn initialize_with<F, R>(&'static self, init: T, f: F) -> R
    where
        F: FnOnce(Option<T>, &T) -> R,
    {
        let (p, rest) = self.slot(Some(init));
        if (p as usize) == 0 {
            crate::rt::panic_str(
                "cannot access a Thread Local Storage value during or after destruction: AccessError",
                file!(),
                line!(),
                column!(),
            );
        }
        f(rest, unsafe { &*p })
    }
}

impl<T: 'static> LocalKey<Cell<T>> {
    /// Sets or initializes the contained value.
    #[track_caller]
    pub fn set(&'static self, value: T) {
        self.initialize_with(Cell::new(value), |value, cell| {
            if let Some(value) = value {
                cell.set(value.into_inner());
            }
        });
    }

    /// Returns a copy of the contained value.
    #[track_caller]
    pub fn get(&'static self) -> T
    where
        T: Copy,
    {
        self.with(|cell| cell.get())
    }

    /// Takes the contained value, leaving `Default::default()` in its place.
    #[track_caller]
    pub fn take(&'static self) -> T
    where
        T: Default,
    {
        self.with(|cell| cell.take())
    }

    /// Replaces the contained value, returning the old value.
    #[track_caller]
    pub fn replace(&'static self, value: T) -> T {
        self.with(|cell| cell.replace(value))
    }

    /// Updates the contained value using a function.
    #[track_caller]
    pub fn update(&'static self, f: impl FnOnce(T) -> T)
    where
        T: Copy,
    {
        self.with(|cell| cell.set(f(cell.get())))
    }
}

impl<T: 'static> LocalKey<RefCell<T>> {
    /// Acquires a reference to the contained value.
    #[track_caller]
    pub fn with_borrow<F, R>(&'static self, f: F) -> R
    where
        F: FnOnce(&T) -> R,
    {
        self.with(|cell| f(&*cell.borrow()))
    }

    /// Acquires a mutable reference to the contained value.
    #[track_caller]
    pub fn with_borrow_mut<F, R>(&'static self, f: F) -> R
    where
        F: FnOnce(&mut T) -> R,
    {
        self.with(|cell| f(&mut *cell.borrow_mut()))
    }

    /// Sets or initializes the contained value.
    #[track_caller]
    pub fn set(&'static self, value: T) {
        self.initialize_with(RefCell::new(value), |value, cell| {
            if let Some(value) = value {
                *cell.borrow_mut() = value.into_inner();
            }
        });
    }

    /// Takes the contained value, leaving `Default::default()` in its place.
    #[track_caller]
    pub fn take(&'static self) -> T
    where
        T: Default,
    {
        self.with(|cell| cell.take())
    }

    /// Replaces the contained value, returning the old value.
    #[track_caller]
    pub fn replace(&'static self, value: T) -> T {
        self.with(|cell| cell.replace(value))
    }
}

/// Declare a new thread local storage key of type `std::thread::LocalKey`.
#[macro_export]
macro_rules! thread_local {
    () => {};
    ($(#[$attr:meta])* $vis:vis static $name:ident : $t:ty = const { $init:expr } ; $($rest:tt)*) => {
        $(#[$attr])* $vis static $name: $crate::thread::LocalKey<$t> = $crate::thread::LocalKey::new(|| $init);
        thread_local!($($rest)*);
    };
    ($(#[$attr:meta])* $vis:vis static $name:ident : $t:ty = const { $init:expr }) => {
        $(#[$attr])* $vis static $name: $crate::thread::LocalKey<$t> = $crate::thread::LocalKey::new(|| $init);
    };
    ($(#[$attr:meta])* $vis:vis static $name:ident : $t:ty = const $init:block ; $($rest:tt)*) => {
        $(#[$attr])* $vis static $name: $crate::thread::LocalKey<$t> = $crate::thread::LocalKey::new(|| $init);
        thread_local!($($rest)*);
    };
    ($(#[$attr:meta])* $vis:vis static $name:ident : $t:ty = const $init:block) => {
        $(#[$attr])* $vis static $name: $crate::thread::LocalKey<$t> = $crate::thread::LocalKey::new(|| $init);
    };
    ($(#[$attr:meta])* $vis:vis static $name:ident : $t:ty = $init:expr ; $($rest:tt)*) => {
        $(#[$attr])* $vis static $name: $crate::thread::LocalKey<$t> = $crate::thread::LocalKey::new(|| $init);
        thread_local!($($rest)*);
    };
    ($(#[$attr:meta])* $vis:vis static $name:ident : $t:ty = $init:expr) => {
        $(#[$attr])* $vis static $name: $crate::thread::LocalKey<$t> = $crate::thread::LocalKey::new(|| $init);
    };
}
