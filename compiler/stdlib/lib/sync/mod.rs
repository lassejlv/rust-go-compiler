//! Synchronization primitives (futex-based, like the real standard library
//! on Linux).

mod arc;
pub mod atomic;
mod barrier;
mod condvar;
pub mod mpsc;
mod mutex;
mod once;
mod poison;
mod rwlock;
pub(crate) mod sys;

pub use self::arc::{Arc, Weak};
pub use self::barrier::{Barrier, BarrierWaitResult};
pub use self::condvar::{Condvar, WaitTimeoutResult};
pub use self::mutex::{Mutex, MutexGuard};
pub use self::once::{LazyLock, Once, OnceLock, OnceState, ONCE_INIT};
pub use self::poison::{LockResult, PoisonError, TryLockError, TryLockResult};
pub use self::rwlock::{RwLock, RwLockReadGuard, RwLockWriteGuard};
