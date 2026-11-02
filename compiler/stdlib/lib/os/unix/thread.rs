//! Unix-specific extensions to primitives in the `std::thread` module.

/// Unix-specific extensions to `JoinHandle`.
pub trait JoinHandleExt {
    /// Extracts the raw pthread_t without taking ownership.
    fn as_pthread_t(&self) -> u64;
    /// Consumes the thread, returning the raw pthread_t.
    fn into_pthread_t(self) -> u64;
}
