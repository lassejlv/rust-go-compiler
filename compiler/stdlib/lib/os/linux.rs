//! Linux-specific definitions.

/// Linux-specific extensions to primitives in the `std::fs` module.
pub mod fs {
    pub use crate::os::unix::fs::MetadataExt;
}

/// Linux-specific extensions to primitives in the `std::process` module.
pub mod process {
    pub use crate::os::unix::process::CommandExt;
}

/// Linux-specific raw type definitions.
pub mod raw {
    pub type dev_t = u64;
    pub type ino_t = u64;
    pub type mode_t = u32;
    pub type nlink_t = u64;
    pub type off_t = i64;
    pub type time_t = i64;
    pub type blkcnt_t = u64;
    pub type blksize_t = u64;
    pub type pthread_t = u64;
}
