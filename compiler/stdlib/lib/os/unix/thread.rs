//! Unix-specific extensions to primitives in the `std::thread` module.

/// The raw pthread handle type. (In rustgo this is the runtime's opaque
/// thread handle.)
#[allow(non_camel_case_types)]
pub type RawPthread = u64;

/// Unix-specific extensions to `JoinHandle`.
pub trait JoinHandleExt {
    /// Extracts the raw pthread_t without taking ownership.
    fn as_pthread_t(&self) -> RawPthread;
    /// Consumes the thread, returning the raw pthread_t.
    fn into_pthread_t(self) -> RawPthread;
}
