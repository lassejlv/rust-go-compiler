//! Unix-specific extensions to primitives in the `std::fs` module.

use crate::ffi::OsStr;
use crate::fs::{self, DirBuilder, DirEntry, File, FileType, Metadata, OpenOptions, Permissions};
use crate::io;
use crate::os::fd::{AsFd, AsRawFd};
use crate::path::Path;
use crate::sys;

/// Unix-specific extensions to `fs::File`.
pub trait FileExt {
    /// Reads a number of bytes starting from a given offset.
    fn read_at(&self, buf: &mut [u8], offset: u64) -> io::Result<usize>;

    /// Reads the exact number of bytes required to fill `buf` from the given offset.
    fn read_exact_at(&self, mut buf: &mut [u8], mut offset: u64) -> io::Result<()> {
        while !buf.is_empty() {
            match self.read_at(buf, offset) {
                Ok(0) => break,
                Ok(n) => {
                    let tmp = buf;
                    buf = &mut tmp[n..];
                    offset += n as u64;
                }
                Err(ref e) if e.kind() == io::ErrorKind::Interrupted => {}
                Err(e) => return Err(e),
            }
        }
        if !buf.is_empty() {
            Err(io::const_io_error(io::ErrorKind::UnexpectedEof, "failed to fill whole buffer"))
        } else {
            Ok(())
        }
    }

    /// Writes a number of bytes starting from a given offset.
    fn write_at(&self, buf: &[u8], offset: u64) -> io::Result<usize>;

    /// Attempts to write an entire buffer starting from a given offset.
    fn write_all_at(&self, mut buf: &[u8], mut offset: u64) -> io::Result<()> {
        while !buf.is_empty() {
            match self.write_at(buf, offset) {
                Ok(0) => {
                    return Err(io::const_io_error(io::ErrorKind::WriteZero, "failed to write whole buffer"));
                }
                Ok(n) => {
                    buf = &buf[n..];
                    offset += n as u64
                }
                Err(ref e) if e.kind() == io::ErrorKind::Interrupted => {}
                Err(e) => return Err(e),
            }
        }
        Ok(())
    }
}

impl FileExt for File {
    fn read_at(&self, buf: &mut [u8], offset: u64) -> io::Result<usize> {
        let r = unsafe { sys::c_pread(self.as_raw_fd(), buf.as_mut_ptr(), buf.len(), offset as i64) };
        sys::cvt_isize(r).map(|n| n as usize)
    }
    fn write_at(&self, buf: &[u8], offset: u64) -> io::Result<usize> {
        let r = unsafe { sys::c_pwrite(self.as_raw_fd(), buf.as_ptr(), buf.len(), offset as i64) };
        sys::cvt_isize(r).map(|n| n as usize)
    }
}

/// Unix-specific extensions to `fs::Permissions`.
pub trait PermissionsExt {
    /// Returns the underlying raw `st_mode` bits.
    fn mode(&self) -> u32;
    /// Sets the underlying raw bits for this set of permissions.
    fn set_mode(&mut self, mode: u32);
    /// Creates a new instance of `Permissions` from the given set of Unix permission bits.
    fn from_mode(mode: u32) -> Self;
}

impl PermissionsExt for Permissions {
    fn mode(&self) -> u32 {
        Permissions::mode(self)
    }
    fn set_mode(&mut self, mode: u32) {
        Permissions::set_mode(self, mode)
    }
    fn from_mode(mode: u32) -> Permissions {
        Permissions::from_mode(mode)
    }
}

/// Unix-specific extensions to `fs::OpenOptions`.
pub trait OpenOptionsExt {
    /// Sets the mode bits that a new file will be created with.
    fn mode(&mut self, mode: u32) -> &mut Self;
    /// Pass custom flags to the `flags` argument of `open`.
    fn custom_flags(&mut self, flags: i32) -> &mut Self;
}

impl OpenOptionsExt for OpenOptions {
    fn mode(&mut self, mode: u32) -> &mut OpenOptions {
        self.set_mode(mode);
        self
    }
    fn custom_flags(&mut self, flags: i32) -> &mut OpenOptions {
        self.set_custom_flags(flags);
        self
    }
}

/// Unix-specific extensions to `fs::Metadata`.
pub trait MetadataExt {
    fn dev(&self) -> u64;
    fn ino(&self) -> u64;
    fn mode(&self) -> u32;
    fn nlink(&self) -> u64;
    fn uid(&self) -> u32;
    fn gid(&self) -> u32;
    fn rdev(&self) -> u64;
    fn size(&self) -> u64;
    fn atime(&self) -> i64;
    fn atime_nsec(&self) -> i64;
    fn mtime(&self) -> i64;
    fn mtime_nsec(&self) -> i64;
    fn ctime(&self) -> i64;
    fn ctime_nsec(&self) -> i64;
    fn blksize(&self) -> u64;
    fn blocks(&self) -> u64;
    // Linux-style names (std::os::linux::fs::MetadataExt)
    fn st_dev(&self) -> u64 {
        self.dev()
    }
    fn st_ino(&self) -> u64 {
        self.ino()
    }
    fn st_mode(&self) -> u32 {
        self.mode()
    }
    fn st_nlink(&self) -> u64 {
        self.nlink()
    }
    fn st_uid(&self) -> u32 {
        self.uid()
    }
    fn st_gid(&self) -> u32 {
        self.gid()
    }
    fn st_rdev(&self) -> u64 {
        self.rdev()
    }
    fn st_size(&self) -> u64 {
        self.size()
    }
    fn st_atime(&self) -> i64 {
        self.atime()
    }
    fn st_atime_nsec(&self) -> i64 {
        self.atime_nsec()
    }
    fn st_mtime(&self) -> i64 {
        self.mtime()
    }
    fn st_mtime_nsec(&self) -> i64 {
        self.mtime_nsec()
    }
    fn st_ctime(&self) -> i64 {
        self.ctime()
    }
    fn st_ctime_nsec(&self) -> i64 {
        self.ctime_nsec()
    }
    fn st_blksize(&self) -> u64 {
        self.blksize()
    }
    fn st_blocks(&self) -> u64 {
        self.blocks()
    }
}

impl MetadataExt for Metadata {
    fn dev(&self) -> u64 {
        self.raw().st_dev
    }
    fn ino(&self) -> u64 {
        self.raw().st_ino
    }
    fn mode(&self) -> u32 {
        self.raw().st_mode
    }
    fn nlink(&self) -> u64 {
        self.raw().st_nlink
    }
    fn uid(&self) -> u32 {
        self.raw().st_uid
    }
    fn gid(&self) -> u32 {
        self.raw().st_gid
    }
    fn rdev(&self) -> u64 {
        self.raw().st_rdev
    }
    fn size(&self) -> u64 {
        self.raw().st_size as u64
    }
    fn atime(&self) -> i64 {
        self.raw().st_atime
    }
    fn atime_nsec(&self) -> i64 {
        self.raw().st_atime_nsec
    }
    fn mtime(&self) -> i64 {
        self.raw().st_mtime
    }
    fn mtime_nsec(&self) -> i64 {
        self.raw().st_mtime_nsec
    }
    fn ctime(&self) -> i64 {
        self.raw().st_ctime
    }
    fn ctime_nsec(&self) -> i64 {
        self.raw().st_ctime_nsec
    }
    fn blksize(&self) -> u64 {
        self.raw().st_blksize as u64
    }
    fn blocks(&self) -> u64 {
        self.raw().st_blocks as u64
    }
}

/// Unix-specific extensions for `fs::FileType`.
pub trait FileTypeExt {
    /// Returns `true` if this file type is a block device.
    fn is_block_device(&self) -> bool;
    /// Returns `true` if this file type is a char device.
    fn is_char_device(&self) -> bool;
    /// Returns `true` if this file type is a fifo.
    fn is_fifo(&self) -> bool;
    /// Returns `true` if this file type is a socket.
    fn is_socket(&self) -> bool;
}

impl FileTypeExt for FileType {
    fn is_block_device(&self) -> bool {
        self.is(sys::S_IFBLK)
    }
    fn is_char_device(&self) -> bool {
        self.is(sys::S_IFCHR)
    }
    fn is_fifo(&self) -> bool {
        self.is(sys::S_IFIFO)
    }
    fn is_socket(&self) -> bool {
        self.is(sys::S_IFSOCK)
    }
}

/// Unix-specific extension methods for `fs::DirEntry`.
pub trait DirEntryExt {
    /// Returns the underlying `d_ino` field in the contained `dirent` structure.
    fn ino(&self) -> u64;
}

impl DirEntryExt for DirEntry {
    fn ino(&self) -> u64 {
        DirEntry::ino(self)
    }
}

/// Unix-specific extension methods for `fs::DirEntry` (borrowed file name).
pub trait DirEntryExt2 {
    /// Returns a reference to the underlying `OsStr` of this entry's filename.
    fn file_name_ref(&self) -> &OsStr;
}

impl DirEntryExt2 for DirEntry {
    fn file_name_ref(&self) -> &OsStr {
        OsStr::from_inner(self.name_bytes())
    }
}

/// Unix-specific extensions to `fs::DirBuilder`.
pub trait DirBuilderExt {
    /// Sets the mode to create new directories with.
    fn mode(&mut self, mode: u32) -> &mut Self;
}

impl DirBuilderExt for DirBuilder {
    fn mode(&mut self, mode: u32) -> &mut DirBuilder {
        self.set_mode(mode);
        self
    }
}

/// Creates a new symbolic link on the filesystem.
pub fn symlink<P: AsRef<Path>, Q: AsRef<Path>>(original: P, link: Q) -> io::Result<()> {
    fs::symlink_impl(original.as_ref(), link.as_ref())
}

fn opt_id(id: Option<u32>) -> u32 {
    match id {
        Some(i) => i,
        None => u32::MAX,
    }
}

/// Changes the owner and group of the specified path.
pub fn chown<P: AsRef<Path>>(dir: P, uid: Option<u32>, gid: Option<u32>) -> io::Result<()> {
    let p = sys::cstr(dir.as_ref().as_os_str().as_encoded_bytes())?;
    sys::cvt(unsafe { sys::c_chown(p.as_ptr(), opt_id(uid), opt_id(gid)) })?;
    Ok(())
}

/// Changes the owner and group of the file referenced by the specified open file descriptor.
pub fn fchown<F: AsFd>(fd: F, uid: Option<u32>, gid: Option<u32>) -> io::Result<()> {
    sys::cvt(unsafe { sys::c_fchown(fd.as_fd().as_raw_fd(), opt_id(uid), opt_id(gid)) })?;
    Ok(())
}

/// Changes the owner and group of the specified path, without dereferencing symbolic links.
pub fn lchown<P: AsRef<Path>>(dir: P, uid: Option<u32>, gid: Option<u32>) -> io::Result<()> {
    let p = sys::cstr(dir.as_ref().as_os_str().as_encoded_bytes())?;
    sys::cvt(unsafe { sys::c_lchown(p.as_ptr(), opt_id(uid), opt_id(gid)) })?;
    Ok(())
}
