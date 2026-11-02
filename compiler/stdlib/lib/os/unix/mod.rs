//! Unix-specific extensions.

pub mod ffi;
pub mod fs;
pub mod io;
pub mod net;
pub mod process;
pub mod thread;

/// Unix-specific raw type definitions.
pub mod raw {
    pub type uid_t = u32;
    pub type gid_t = u32;
    pub type pid_t = i32;
    pub use crate::os::linux::raw::*;
}

/// A prelude for conveniently writing platform-specific code.
pub mod prelude {
    pub use super::ffi::{OsStrExt, OsStringExt};
    pub use super::fs::{DirEntryExt, FileExt, FileTypeExt, MetadataExt, OpenOptionsExt, PermissionsExt};
    pub use super::io::{AsFd, AsRawFd, BorrowedFd, FromRawFd, IntoRawFd, OwnedFd, RawFd};
    pub use super::process::{CommandExt, ExitStatusExt};
    pub use super::thread::JoinHandleExt;
}
