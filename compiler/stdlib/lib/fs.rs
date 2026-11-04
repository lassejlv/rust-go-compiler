//! Filesystem manipulation operations.

use crate::ffi::{OsStr, OsString};
use crate::fmt;
use crate::io::{self, IoSlice, IoSliceMut, Read, Seek, SeekFrom, Write};
use crate::os::fd::{AsFd, AsRawFd, BorrowedFd, FromRawFd, IntoRawFd, OwnedFd, RawFd};
use crate::path::{Path, PathBuf};
use crate::sync::Arc;
use crate::sys;
use crate::time::{Duration, SystemTime, UNIX_EPOCH};

// ---------------------------------------------------------------------------
// helpers

fn cpath(p: &Path) -> io::Result<Vec<u8>> {
    sys::cstr(p.as_os_str().as_encoded_bytes())
}

#[repr(C)]
#[derive(Clone, Copy)]
struct StatxTimestamp {
    tv_sec: i64,
    tv_nsec: u32,
    __reserved: i32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct Statx {
    stx_mask: u32,
    stx_blksize: u32,
    stx_attributes: u64,
    stx_nlink: u32,
    stx_uid: u32,
    stx_gid: u32,
    stx_mode: u16,
    __spare0: u16,
    stx_ino: u64,
    stx_size: u64,
    stx_blocks: u64,
    stx_attributes_mask: u64,
    stx_atime: StatxTimestamp,
    stx_btime: StatxTimestamp,
    stx_ctime: StatxTimestamp,
    stx_mtime: StatxTimestamp,
    stx_rdev_major: u32,
    stx_rdev_minor: u32,
    stx_dev_major: u32,
    stx_dev_minor: u32,
    __spare2: [u64; 14],
}

extern "C" {
    #[link_name = "statx"]
    fn c_statx(dirfd: i32, path: *const u8, flags: i32, mask: u32, buf: *mut Statx) -> i32;
}

const AT_EMPTY_PATH: i32 = 0x1000;
const STATX_ALL: u32 = 0xfff;
const STATX_BTIME: u32 = 0x800;

fn makedev(major: u32, minor: u32) -> u64 {
    let major = major as u64;
    let minor = minor as u64;
    ((major & 0xfffff000) << 32) | ((major & 0xfff) << 8) | ((minor & 0xffffff00) << 12) | (minor & 0xff)
}

// Runs statx (falls back to the stat family when it is unavailable).
fn do_stat(fd: i32, path: &[u8], flags: i32) -> io::Result<Metadata> {
    let mut sx: Statx = unsafe { crate::mem::zeroed() };
    let r = unsafe { c_statx(fd, path.as_ptr(), flags, STATX_ALL, &mut sx) };
    if r == 0 {
        let mut st = sys::stat_t::zeroed();
        st.st_dev = makedev(sx.stx_dev_major, sx.stx_dev_minor);
        st.st_ino = sx.stx_ino;
        st.st_nlink = sx.stx_nlink as u64;
        st.st_mode = sx.stx_mode as u32;
        st.st_uid = sx.stx_uid;
        st.st_gid = sx.stx_gid;
        st.st_rdev = makedev(sx.stx_rdev_major, sx.stx_rdev_minor);
        st.st_size = sx.stx_size as i64;
        st.st_blksize = sx.stx_blksize as i64;
        st.st_blocks = sx.stx_blocks as i64;
        st.st_atime = sx.stx_atime.tv_sec;
        st.st_atime_nsec = sx.stx_atime.tv_nsec as i64;
        st.st_mtime = sx.stx_mtime.tv_sec;
        st.st_mtime_nsec = sx.stx_mtime.tv_nsec as i64;
        st.st_ctime = sx.stx_ctime.tv_sec;
        st.st_ctime_nsec = sx.stx_ctime.tv_nsec as i64;
        let btime = if sx.stx_mask & STATX_BTIME != 0 {
            Some((sx.stx_btime.tv_sec, sx.stx_btime.tv_nsec as i64))
        } else {
            None
        };
        return Ok(Metadata { stat: st, btime, has_statx: true });
    }
    let e = sys::errno();
    if e != sys::ENOSYS && e != sys::EPERM {
        return Err(io::Error::from_raw_os_error(e));
    }
    // fallback
    let mut st = sys::stat_t::zeroed();
    unsafe {
        if path.len() <= 1 && fd >= 0 {
            sys::cvt(sys::c_fstat(fd, &mut st))?;
        } else if flags & sys::AT_SYMLINK_NOFOLLOW != 0 {
            sys::cvt(sys::c_lstat(path.as_ptr(), &mut st))?;
        } else {
            sys::cvt(sys::c_stat(path.as_ptr(), &mut st))?;
        }
    }
    Ok(Metadata { stat: st, btime: None, has_statx: false })
}

fn system_time(sec: i64, nsec: i64) -> SystemTime {
    if sec >= 0 {
        UNIX_EPOCH + Duration::new(sec as u64, nsec as u32)
    } else {
        UNIX_EPOCH
    }
}

// ---------------------------------------------------------------------------
// File

/// An object providing access to an open file on the filesystem.
pub struct File {
    inner: OwnedFd,
}

impl File {
    /// Attempts to open a file in read-only mode.
    pub fn open<P: AsRef<Path>>(path: P) -> io::Result<File> {
        OpenOptions::new().read(true).open(path.as_ref())
    }

    /// Opens a file in write-only mode (creating or truncating it).
    pub fn create<P: AsRef<Path>>(path: P) -> io::Result<File> {
        OpenOptions::new().write(true).create(true).truncate(true).open(path.as_ref())
    }

    /// Creates a new file in read-write mode; error if the file exists.
    pub fn create_new<P: AsRef<Path>>(path: P) -> io::Result<File> {
        OpenOptions::new().read(true).write(true).create_new(true).open(path.as_ref())
    }

    /// Returns a new `OpenOptions` object.
    #[inline]
    pub fn options() -> OpenOptions {
        OpenOptions::new()
    }

    /// Attempts to sync all OS-internal file content and metadata to disk.
    pub fn sync_all(&self) -> io::Result<()> {
        sys::cvt_r(|| unsafe { sys::c_fsync(self.inner.raw()) })?;
        Ok(())
    }

    /// Like `sync_all`, except that it might not synchronize file metadata.
    pub fn sync_data(&self) -> io::Result<()> {
        sys::cvt_r(|| unsafe { sys::c_fdatasync(self.inner.raw()) })?;
        Ok(())
    }

    /// Truncates or extends the underlying file, updating the size of this file to become `size`.
    pub fn set_len(&self, size: u64) -> io::Result<()> {
        if size > i64::MAX as u64 {
            return Err(io::const_io_error(io::ErrorKind::InvalidInput, "cannot set file length beyond i64::MAX"));
        }
        sys::cvt_r(|| unsafe { sys::c_ftruncate(self.inner.raw(), size as i64) })?;
        Ok(())
    }

    /// Queries metadata about the underlying file.
    pub fn metadata(&self) -> io::Result<Metadata> {
        do_stat(self.inner.raw(), b"\0", AT_EMPTY_PATH)
    }

    /// Creates a new `File` instance that shares the same underlying file handle as the existing `File` instance.
    pub fn try_clone(&self) -> io::Result<File> {
        Ok(File { inner: self.inner.try_clone()? })
    }

    /// Changes the permissions on the underlying file.
    pub fn set_permissions(&self, perm: Permissions) -> io::Result<()> {
        sys::cvt_r(|| unsafe { sys::c_fchmod(self.inner.raw(), perm.mode) })?;
        Ok(())
    }

    /// Changes the timestamps of the underlying file.
    pub fn set_times(&self, times: FileTimes) -> io::Result<()> {
        let to_ts = |t: Option<SystemTime>| -> sys::timespec_t {
            match t {
                Some(t) => {
                    let d = t.duration_since(UNIX_EPOCH).unwrap_or(Duration::ZERO);
                    sys::timespec_t { tv_sec: d.as_secs() as i64, tv_nsec: d.subsec_nanos() as i64 }
                }
                None => sys::timespec_t { tv_sec: 0, tv_nsec: sys::UTIME_OMIT },
            }
        };
        let ts = [to_ts(times.accessed), to_ts(times.modified)];
        sys::cvt(unsafe { sys::c_futimens(self.inner.raw(), ts.as_ptr()) })?;
        Ok(())
    }

    /// Changes the modification time of the underlying file.
    pub fn set_modified(&self, time: SystemTime) -> io::Result<()> {
        self.set_times(FileTimes::new().set_modified(time))
    }

    fn seek_raw(&self, pos: SeekFrom) -> io::Result<u64> {
        let (whence, pos) = match pos {
            SeekFrom::Start(off) => (sys::SEEK_SET, off as i64),
            SeekFrom::End(off) => (sys::SEEK_END, off),
            SeekFrom::Current(off) => (sys::SEEK_CUR, off),
        };
        let n = sys::cvt_i64(unsafe { sys::c_lseek(self.inner.raw(), pos, whence) })?;
        Ok(n as u64)
    }

    fn read_at_raw(&self, buf: &mut [u8], offset: u64) -> io::Result<usize> {
        let r = unsafe { sys::c_pread(self.inner.raw(), buf.as_mut_ptr(), buf.len(), offset as i64) };
        sys::cvt_isize(r).map(|n| n as usize)
    }

    fn write_at_raw(&self, buf: &[u8], offset: u64) -> io::Result<usize> {
        let r = unsafe { sys::c_pwrite(self.inner.raw(), buf.as_ptr(), buf.len(), offset as i64) };
        sys::cvt_isize(r).map(|n| n as usize)
    }

    fn read_to_end_raw(&self, buf: &mut Vec<u8>) -> io::Result<usize> {
        // size hint from metadata + current position
        let hint = match self.metadata() {
            Ok(m) => {
                let pos = unsafe { sys::c_lseek(self.inner.raw(), 0, sys::SEEK_CUR) };
                if pos >= 0 && m.len() >= pos as u64 { (m.len() - pos as u64) as usize } else { 0 }
            }
            Err(_) => 0,
        };
        buf.reserve(hint);
        let start = buf.len();
        let mut chunk = [0u8; 8192];
        loop {
            let n = self.inner.read(&mut chunk)?;
            if n == 0 {
                return Ok(buf.len() - start);
            }
            buf.extend_from_slice(&chunk[..n]);
        }
    }

    /// Acquire an exclusive advisory lock on the file.
    pub fn lock(&self) -> io::Result<()> {
        sys::cvt(unsafe { c_flock(self.inner.raw(), 2) })?;
        Ok(())
    }

    /// Acquire a shared advisory lock on the file.
    pub fn lock_shared(&self) -> io::Result<()> {
        sys::cvt(unsafe { c_flock(self.inner.raw(), 1) })?;
        Ok(())
    }

    /// Try to acquire an exclusive advisory lock on the file without blocking.
    pub fn try_lock(&self) -> Result<(), TryLockError> {
        self.try_flock(2)
    }

    /// Try to acquire a shared advisory lock on the file without blocking.
    pub fn try_lock_shared(&self) -> Result<(), TryLockError> {
        self.try_flock(1)
    }

    fn try_flock(&self, op: i32) -> Result<(), TryLockError> {
        // LOCK_NB = 4
        if unsafe { c_flock(self.inner.raw(), op | 4) } == 0 {
            return Ok(());
        }
        let err = io::Error::last_os_error();
        if err.kind() == io::ErrorKind::WouldBlock {
            Err(TryLockError::WouldBlock)
        } else {
            Err(TryLockError::Error(err))
        }
    }

    /// Release all locks on the file.
    pub fn unlock(&self) -> io::Result<()> {
        sys::cvt(unsafe { c_flock(self.inner.raw(), 8) })?;
        Ok(())
    }
}

/// Errors which can be returned by `File::try_lock` and `File::try_lock_shared`.
pub enum TryLockError {
    /// The lock could not be acquired due to an I/O error on the file.
    Error(io::Error),
    /// The lock could not be acquired at this time because it is held by another handle/process.
    WouldBlock,
}

impl fmt::Debug for TryLockError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TryLockError::Error(err) => fmt::Debug::fmt(err, f),
            TryLockError::WouldBlock => fmt::Debug::fmt("WouldBlock", f),
        }
    }
}

impl fmt::Display for TryLockError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            TryLockError::Error(_) => "lock acquisition failed due to I/O error",
            TryLockError::WouldBlock => "lock acquisition failed because the operation would block",
        };
        fmt::Display::fmt(s, f)
    }
}

impl crate::error::Error for TryLockError {}

impl From<TryLockError> for io::Error {
    fn from(err: TryLockError) -> io::Error {
        match err {
            TryLockError::Error(err) => err,
            TryLockError::WouldBlock => io::ErrorKind::WouldBlock.into(),
        }
    }
}

extern "C" {
    #[link_name = "flock"]
    fn c_flock(fd: i32, op: i32) -> i32;
}

impl Read for File {
    #[inline]
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.inner.read(buf)
    }
    fn read_vectored(&mut self, bufs: &mut [IoSliceMut<'_>]) -> io::Result<usize> {
        self.inner.read_vectored(bufs)
    }
    fn is_read_vectored(&self) -> bool {
        true
    }
    fn read_to_end(&mut self, buf: &mut Vec<u8>) -> io::Result<usize> {
        self.read_to_end_raw(buf)
    }
}

impl Write for File {
    #[inline]
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.inner.write(buf)
    }
    fn write_vectored(&mut self, bufs: &[IoSlice<'_>]) -> io::Result<usize> {
        self.inner.write_vectored(bufs)
    }
    fn is_write_vectored(&self) -> bool {
        true
    }
    #[inline]
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Seek for File {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        self.seek_raw(pos)
    }
}

impl Read for &File {
    #[inline]
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.inner.read(buf)
    }
    fn read_vectored(&mut self, bufs: &mut [IoSliceMut<'_>]) -> io::Result<usize> {
        self.inner.read_vectored(bufs)
    }
    fn read_to_end(&mut self, buf: &mut Vec<u8>) -> io::Result<usize> {
        self.read_to_end_raw(buf)
    }
}

impl Write for &File {
    #[inline]
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.inner.write(buf)
    }
    fn write_vectored(&mut self, bufs: &[IoSlice<'_>]) -> io::Result<usize> {
        self.inner.write_vectored(bufs)
    }
    #[inline]
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Seek for &File {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        self.seek_raw(pos)
    }
}

impl Read for Arc<File> {
    #[inline]
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.inner.read(buf)
    }
}

impl Write for Arc<File> {
    #[inline]
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.inner.write(buf)
    }
    #[inline]
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Seek for Arc<File> {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        self.seek_raw(pos)
    }
}

fn fd_debug(fd: RawFd, name: &str, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let mut b = f.debug_struct(name);
    b.field("fd", &fd);
    let mut p = OsString::from("/proc/self/fd/");
    p.push(fd.to_string());
    if let Ok(path) = read_link(Path::new(&p)) {
        b.field("path", &path);
    }
    let mode = unsafe { sys::c_fcntl(fd, sys::F_GETFL) };
    if mode != -1 {
        let (r, w) = match mode & sys::O_ACCMODE {
            sys::O_RDONLY => (true, false),
            sys::O_RDWR => (true, true),
            sys::O_WRONLY => (false, true),
            _ => (false, false),
        };
        b.field("read", &r).field("write", &w);
    }
    b.finish()
}

impl fmt::Debug for File {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fd_debug(self.inner.raw(), "File", f)
    }
}

impl AsRawFd for File {
    #[inline]
    fn as_raw_fd(&self) -> RawFd {
        self.inner.raw()
    }
}

impl AsFd for File {
    #[inline]
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.inner.as_fd()
    }
}

impl IntoRawFd for File {
    #[inline]
    fn into_raw_fd(self) -> RawFd {
        self.inner.into_raw_fd()
    }
}

impl FromRawFd for File {
    #[inline]
    unsafe fn from_raw_fd(fd: RawFd) -> File {
        File { inner: OwnedFd::from_raw_fd(fd) }
    }
}

impl From<OwnedFd> for File {
    #[inline]
    fn from(fd: OwnedFd) -> File {
        File { inner: fd }
    }
}

impl From<File> for OwnedFd {
    #[inline]
    fn from(f: File) -> OwnedFd {
        f.inner
    }
}

impl io::IsTerminal for File {
    fn is_terminal(&self) -> bool {
        io::stdio_is_terminal(self.inner.raw())
    }
}

// ---------------------------------------------------------------------------
// OpenOptions

/// Options and flags which can be used to configure how a file is opened.
#[derive(Clone)]
pub struct OpenOptions {
    read: bool,
    write: bool,
    append: bool,
    truncate: bool,
    create: bool,
    create_new: bool,
    custom_flags: i32,
    mode: u32,
}

impl OpenOptions {
    /// Creates a blank new set of options ready for configuration.
    pub fn new() -> OpenOptions {
        OpenOptions {
            read: false,
            write: false,
            append: false,
            truncate: false,
            create: false,
            create_new: false,
            custom_flags: 0,
            mode: 0o666,
        }
    }

    /// Sets the option for read access.
    pub fn read(&mut self, read: bool) -> &mut OpenOptions {
        self.read = read;
        self
    }

    /// Sets the option for write access.
    pub fn write(&mut self, write: bool) -> &mut OpenOptions {
        self.write = write;
        self
    }

    /// Sets the option for the append mode.
    pub fn append(&mut self, append: bool) -> &mut OpenOptions {
        self.append = append;
        self
    }

    /// Sets the option for truncating a previous file.
    pub fn truncate(&mut self, truncate: bool) -> &mut OpenOptions {
        self.truncate = truncate;
        self
    }

    /// Sets the option to create a new file, or open it if it already exists.
    pub fn create(&mut self, create: bool) -> &mut OpenOptions {
        self.create = create;
        self
    }

    /// Sets the option to create a new file, failing if it already exists.
    pub fn create_new(&mut self, create_new: bool) -> &mut OpenOptions {
        self.create_new = create_new;
        self
    }

    pub(crate) fn set_mode(&mut self, mode: u32) {
        self.mode = mode;
    }

    pub(crate) fn set_custom_flags(&mut self, flags: i32) {
        self.custom_flags = flags;
    }

    fn get_access_mode(&self) -> io::Result<i32> {
        match (self.read, self.write, self.append) {
            (true, false, false) => Ok(sys::O_RDONLY),
            (false, true, false) => Ok(sys::O_WRONLY),
            (true, true, false) => Ok(sys::O_RDWR),
            (false, _, true) => Ok(sys::O_WRONLY | sys::O_APPEND),
            (true, _, true) => Ok(sys::O_RDWR | sys::O_APPEND),
            (false, false, false) => Err(io::Error::from_raw_os_error(sys::EINVAL)),
        }
    }

    fn get_creation_mode(&self) -> io::Result<i32> {
        match (self.write, self.append) {
            (true, false) => {}
            (false, false) => {
                if self.truncate || self.create || self.create_new {
                    return Err(io::Error::from_raw_os_error(sys::EINVAL));
                }
            }
            (_, true) => {
                if self.truncate && !self.create_new {
                    return Err(io::Error::from_raw_os_error(sys::EINVAL));
                }
            }
        }
        Ok(match (self.create, self.truncate, self.create_new) {
            (false, false, false) => 0,
            (true, false, false) => sys::O_CREAT,
            (false, true, false) => sys::O_TRUNC,
            (true, true, false) => sys::O_CREAT | sys::O_TRUNC,
            (_, _, true) => sys::O_CREAT | sys::O_EXCL,
        })
    }

    /// Opens a file at `path` with the options specified by `self`.
    pub fn open<P: AsRef<Path>>(&self, path: P) -> io::Result<File> {
        self._open(path.as_ref())
    }

    fn _open(&self, path: &Path) -> io::Result<File> {
        let p = cpath(path)?;
        let flags = sys::O_CLOEXEC
            | self.get_access_mode()?
            | self.get_creation_mode()?
            | (self.custom_flags & !sys::O_ACCMODE);
        let mode = self.mode;
        let fd = sys::cvt_r(|| unsafe { sys::c_open(p.as_ptr(), flags, mode) })?;
        Ok(File { inner: unsafe { OwnedFd::from_raw_fd(fd) } })
    }
}

impl fmt::Debug for OpenOptions {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        struct Inner<'a>(&'a OpenOptions);
        impl<'a> fmt::Debug for Inner<'a> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                let o = self.0;
                f.debug_struct("OpenOptions")
                    .field("read", &o.read)
                    .field("write", &o.write)
                    .field("append", &o.append)
                    .field("truncate", &o.truncate)
                    .field("create", &o.create)
                    .field("create_new", &o.create_new)
                    .field("custom_flags", &o.custom_flags)
                    .field("mode", &ModeDebug(o.mode))
                    .finish()
            }
        }
        f.debug_tuple("OpenOptions").field(&Inner(self)).finish()
    }
}

impl Default for OpenOptions {
    fn default() -> Self {
        OpenOptions::new()
    }
}

// ---------------------------------------------------------------------------
// Metadata, FileType, Permissions, FileTimes

/// Metadata information about a file.
#[derive(Clone)]
pub struct Metadata {
    stat: sys::stat_t,
    btime: Option<(i64, i64)>,
    has_statx: bool,
}

impl Metadata {
    /// Returns the file type for this metadata.
    pub fn file_type(&self) -> FileType {
        FileType { mode: self.stat.st_mode }
    }

    /// Returns `true` if this metadata is for a directory.
    pub fn is_dir(&self) -> bool {
        self.file_type().is_dir()
    }

    /// Returns `true` if this metadata is for a regular file.
    pub fn is_file(&self) -> bool {
        self.file_type().is_file()
    }

    /// Returns `true` if this metadata is for a symbolic link.
    pub fn is_symlink(&self) -> bool {
        self.file_type().is_symlink()
    }

    /// Returns the size of the file, in bytes, this metadata is for.
    pub fn len(&self) -> u64 {
        self.stat.st_size as u64
    }

    /// Returns the permissions of the file this metadata is for.
    pub fn permissions(&self) -> Permissions {
        Permissions { mode: self.stat.st_mode }
    }

    /// Returns the last modification time listed in this metadata.
    pub fn modified(&self) -> io::Result<SystemTime> {
        Ok(system_time(self.stat.st_mtime, self.stat.st_mtime_nsec))
    }

    /// Returns the last access time of this metadata.
    pub fn accessed(&self) -> io::Result<SystemTime> {
        Ok(system_time(self.stat.st_atime, self.stat.st_atime_nsec))
    }

    /// Returns the creation time listed in this metadata.
    pub fn created(&self) -> io::Result<SystemTime> {
        match self.btime {
            Some((s, ns)) => Ok(system_time(s, ns)),
            None => {
                if self.has_statx {
                    Err(io::const_io_error(
                        io::ErrorKind::Unsupported,
                        "creation time is not available for the filesystem",
                    ))
                } else {
                    Err(io::const_io_error(
                        io::ErrorKind::Unsupported,
                        "creation time is not available on this platform currently",
                    ))
                }
            }
        }
    }

    pub(crate) fn raw(&self) -> &sys::stat_t {
        &self.stat
    }
}

impl fmt::Debug for Metadata {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut debug = f.debug_struct("Metadata");
        debug.field("file_type", &self.file_type());
        debug.field("permissions", &self.permissions());
        debug.field("len", &self.len());
        if let Ok(modified) = self.modified() {
            debug.field("modified", &modified);
        }
        if let Ok(accessed) = self.accessed() {
            debug.field("accessed", &accessed);
        }
        if let Ok(created) = self.created() {
            debug.field("created", &created);
        }
        debug.finish_non_exhaustive()
    }
}

/// A structure representing a type of file with accessors for each file type.
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct FileType {
    mode: u32,
}

impl FileType {
    /// Tests whether this file type represents a directory.
    pub fn is_dir(&self) -> bool {
        self.mode & sys::S_IFMT == sys::S_IFDIR
    }

    /// Tests whether this file type represents a regular file.
    pub fn is_file(&self) -> bool {
        self.mode & sys::S_IFMT == sys::S_IFREG
    }

    /// Tests whether this file type represents a symbolic link.
    pub fn is_symlink(&self) -> bool {
        self.mode & sys::S_IFMT == sys::S_IFLNK
    }

    pub(crate) fn is(&self, mode: u32) -> bool {
        self.mode & sys::S_IFMT == mode
    }
}

impl fmt::Debug for FileType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FileType")
            .field("is_file", &self.is_file())
            .field("is_dir", &self.is_dir())
            .field("is_symlink", &self.is_symlink())
            .finish_non_exhaustive()
    }
}

/// Representation of the various permissions on a file.
#[derive(Clone, PartialEq, Eq)]
pub struct Permissions {
    mode: u32,
}

impl Permissions {
    /// Returns `true` if these permissions describe a readonly (unwritable) file.
    pub fn readonly(&self) -> bool {
        self.mode & 0o222 == 0
    }

    /// Modifies the readonly flag for this set of permissions.
    pub fn set_readonly(&mut self, readonly: bool) {
        if readonly {
            self.mode &= !0o222;
        } else {
            self.mode |= 0o222;
        }
    }

    pub(crate) fn mode(&self) -> u32 {
        self.mode
    }

    pub(crate) fn set_mode(&mut self, mode: u32) {
        self.mode = mode;
    }

    pub(crate) fn from_mode(mode: u32) -> Permissions {
        Permissions { mode }
    }
}

struct ModeDebug(u32);

impl fmt::Debug for ModeDebug {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mode = self.0;
        write!(f, "0o{:06o}", mode)?;
        let entry_type = match mode & sys::S_IFMT {
            sys::S_IFDIR => 'd',
            sys::S_IFBLK => 'b',
            sys::S_IFCHR => 'c',
            sys::S_IFLNK => 'l',
            sys::S_IFIFO => 'p',
            sys::S_IFREG => '-',
            _ => return Ok(()),
        };
        f.write_str(" (")?;
        f.write_char(entry_type)?;
        let bit = |f: &mut fmt::Formatter<'_>, m: u32, c: char| -> fmt::Result {
            f.write_char(if mode & m != 0 { c } else { '-' })
        };
        bit(f, 0o400, 'r')?;
        bit(f, 0o200, 'w')?;
        let ux = mode & 0o100 != 0;
        let setuid = mode & 0o4000 != 0;
        f.write_char(match (ux, setuid) {
            (true, false) => 'x',
            (false, false) => '-',
            (true, true) => 's',
            (false, true) => 'S',
        })?;
        bit(f, 0o040, 'r')?;
        bit(f, 0o020, 'w')?;
        let gx = mode & 0o010 != 0;
        let setgid = mode & 0o2000 != 0;
        f.write_char(match (gx, setgid) {
            (true, false) => 'x',
            (false, false) => '-',
            (true, true) => 's',
            (false, true) => 'S',
        })?;
        bit(f, 0o004, 'r')?;
        bit(f, 0o002, 'w')?;
        let ox = mode & 0o001 != 0;
        let sticky = mode & 0o1000 != 0;
        f.write_char(match (entry_type, ox, sticky) {
            ('d', true, true) => 't',
            ('d', false, true) => 'T',
            (_, true, _) => 'x',
            (_, false, _) => '-',
        })?;
        f.write_char(')')
    }
}

use crate::fmt::Write as _;

impl fmt::Debug for Permissions {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        struct FilePermissions(u32);
        impl fmt::Debug for FilePermissions {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.debug_struct("FilePermissions").field("mode", &ModeDebug(self.0)).finish()
            }
        }
        f.debug_tuple("Permissions").field(&FilePermissions(self.mode)).finish()
    }
}

/// Representation of the various timestamps on a file.
#[derive(Copy, Clone, Debug, Default)]
pub struct FileTimes {
    accessed: Option<SystemTime>,
    modified: Option<SystemTime>,
}

impl FileTimes {
    /// Creates a new `FileTimes` with no times set.
    pub fn new() -> Self {
        FileTimes { accessed: None, modified: None }
    }

    /// Set the last access time of a file.
    pub fn set_accessed(mut self, t: SystemTime) -> Self {
        self.accessed = Some(t);
        self
    }

    /// Set the last modified time of a file.
    pub fn set_modified(mut self, t: SystemTime) -> Self {
        self.modified = Some(t);
        self
    }
}

// ---------------------------------------------------------------------------
// ReadDir / DirEntry

struct DirHandle(*mut u8);

impl Drop for DirHandle {
    fn drop(&mut self) {
        unsafe {
            sys::c_closedir(self.0);
        }
    }
}

unsafe impl Send for DirHandle {}
unsafe impl Sync for DirHandle {}

/// Iterator over the entries in a directory.
pub struct ReadDir {
    dir: DirHandle,
    root: Arc<PathBuf>,
    end_of_stream: bool,
}

/// Entries returned by the `ReadDir` iterator.
pub struct DirEntry {
    root: Arc<PathBuf>,
    name: Vec<u8>,
    d_type: u8,
    ino: u64,
}

impl Iterator for ReadDir {
    type Item = io::Result<DirEntry>;

    fn next(&mut self) -> Option<io::Result<DirEntry>> {
        if self.end_of_stream {
            return None;
        }
        loop {
            sys::set_errno(0);
            let ent = unsafe { sys::c_readdir(self.dir.0) };
            if ent.is_null() {
                self.end_of_stream = true;
                let e = sys::errno();
                if e != 0 {
                    return Some(Err(io::Error::from_raw_os_error(e)));
                }
                return None;
            }
            let (name, d_type, ino) = unsafe {
                let namep = (ent as *const u8).add(19);
                (sys::from_cstr(namep), (*ent).d_type, (*ent).d_ino)
            };
            if name == b"." || name == b".." {
                continue;
            }
            return Some(Ok(DirEntry { root: self.root.clone(), name, d_type, ino }));
        }
    }
}

impl fmt::Debug for ReadDir {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("ReadDir").field(&*self.root).finish()
    }
}

impl DirEntry {
    /// Returns the full path to the file that this entry represents.
    pub fn path(&self) -> PathBuf {
        self.root.join(OsStr::from_inner(&self.name))
    }

    /// Returns the metadata for the file that this entry points at (does not traverse symlinks).
    pub fn metadata(&self) -> io::Result<Metadata> {
        symlink_metadata(&self.path())
    }

    /// Returns the file type for the file that this entry points at.
    pub fn file_type(&self) -> io::Result<FileType> {
        let mode = match self.d_type {
            sys::DT_CHR => sys::S_IFCHR,
            sys::DT_FIFO => sys::S_IFIFO,
            sys::DT_LNK => sys::S_IFLNK,
            sys::DT_REG => sys::S_IFREG,
            sys::DT_SOCK => sys::S_IFSOCK,
            sys::DT_DIR => sys::S_IFDIR,
            sys::DT_BLK => sys::S_IFBLK,
            _ => return self.metadata().map(|m| m.file_type()),
        };
        Ok(FileType { mode })
    }

    /// Returns the file name of this directory entry without any leading path component(s).
    pub fn file_name(&self) -> OsString {
        OsString::from_vec(self.name.clone())
    }

    pub(crate) fn ino(&self) -> u64 {
        self.ino
    }

    pub(crate) fn name_bytes(&self) -> &[u8] {
        &self.name
    }
}

impl fmt::Debug for DirEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("DirEntry").field(&self.path()).finish()
    }
}

/// Returns an iterator over the entries within a directory.
pub fn read_dir<P: AsRef<Path>>(path: P) -> io::Result<ReadDir> {
    let path = path.as_ref();
    let p = cpath(path)?;
    let d = unsafe { sys::c_opendir(p.as_ptr()) };
    if d.is_null() {
        return Err(io::Error::last_os_error());
    }
    Ok(ReadDir { dir: DirHandle(d), root: Arc::new(path.to_path_buf()), end_of_stream: false })
}

// ---------------------------------------------------------------------------
// DirBuilder

/// A builder used to create directories in various manners.
pub struct DirBuilder {
    recursive: bool,
    mode: u32,
}

impl DirBuilder {
    /// Creates a new set of options with default mode/security settings for all platforms and also non-recursive.
    pub fn new() -> DirBuilder {
        DirBuilder { recursive: false, mode: 0o777 }
    }

    /// Indicates that directories should be created recursively.
    pub fn recursive(&mut self, recursive: bool) -> &mut Self {
        self.recursive = recursive;
        self
    }

    pub(crate) fn set_mode(&mut self, mode: u32) {
        self.mode = mode;
    }

    /// Creates the specified directory with the options configured in this builder.
    pub fn create<P: AsRef<Path>>(&self, path: P) -> io::Result<()> {
        self._create(path.as_ref())
    }

    fn _create(&self, path: &Path) -> io::Result<()> {
        if self.recursive { self.create_dir_all(path) } else { self.mkdir(path) }
    }

    fn mkdir(&self, path: &Path) -> io::Result<()> {
        let p = cpath(path)?;
        sys::cvt(unsafe { sys::c_mkdir(p.as_ptr(), self.mode) })?;
        Ok(())
    }

    fn create_dir_all(&self, path: &Path) -> io::Result<()> {
        if path == Path::new("") {
            return Ok(());
        }
        match self.mkdir(path) {
            Ok(()) => return Ok(()),
            Err(ref e) if e.kind() == io::ErrorKind::NotFound => {}
            Err(_) if path.is_dir() => return Ok(()),
            Err(e) => return Err(e),
        }
        match path.parent() {
            Some(p) => self.create_dir_all(p)?,
            None => {
                return Err(io::const_io_error(io::ErrorKind::Uncategorized, "failed to create whole tree"));
            }
        }
        match self.mkdir(path) {
            Ok(()) => Ok(()),
            Err(_) if path.is_dir() => Ok(()),
            Err(e) => Err(e),
        }
    }
}

impl fmt::Debug for DirBuilder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        struct Inner(u32);
        impl fmt::Debug for Inner {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.debug_struct("DirBuilder").field("mode", &ModeDebug(self.0)).finish()
            }
        }
        f.debug_struct("DirBuilder").field("inner", &Inner(self.mode)).field("recursive", &self.recursive).finish()
    }
}

impl Default for DirBuilder {
    fn default() -> Self {
        DirBuilder::new()
    }
}

// ---------------------------------------------------------------------------
// Free functions

/// Reads the entire contents of a file into a bytes vector.
pub fn read<P: AsRef<Path>>(path: P) -> io::Result<Vec<u8>> {
    let mut file = File::open(path.as_ref())?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)?;
    Ok(bytes)
}

/// Reads the entire contents of a file into a string.
pub fn read_to_string<P: AsRef<Path>>(path: P) -> io::Result<String> {
    let mut file = File::open(path.as_ref())?;
    let mut string = String::new();
    file.read_to_string(&mut string)?;
    Ok(string)
}

/// Writes a slice as the entire contents of a file.
pub fn write<P: AsRef<Path>, C: AsRef<[u8]>>(path: P, contents: C) -> io::Result<()> {
    File::create(path.as_ref())?.write_all(contents.as_ref())
}

/// Removes a file from the filesystem.
pub fn remove_file<P: AsRef<Path>>(path: P) -> io::Result<()> {
    let p = cpath(path.as_ref())?;
    sys::cvt(unsafe { sys::c_unlink(p.as_ptr()) })?;
    Ok(())
}

/// Given a path, queries the file system to get information about a file, directory, etc.
pub fn metadata<P: AsRef<Path>>(path: P) -> io::Result<Metadata> {
    let p = cpath(path.as_ref())?;
    do_stat(sys::AT_FDCWD, &p, 0)
}

/// Queries the metadata about a file without following symlinks.
pub fn symlink_metadata<P: AsRef<Path>>(path: P) -> io::Result<Metadata> {
    let p = cpath(path.as_ref())?;
    do_stat(sys::AT_FDCWD, &p, sys::AT_SYMLINK_NOFOLLOW)
}

/// Renames a file or directory to a new name, replacing the original file if `to` already exists.
pub fn rename<P: AsRef<Path>, Q: AsRef<Path>>(from: P, to: Q) -> io::Result<()> {
    let a = cpath(from.as_ref())?;
    let b = cpath(to.as_ref())?;
    sys::cvt(unsafe { sys::c_rename(a.as_ptr(), b.as_ptr()) })?;
    Ok(())
}

/// Copies the contents of one file to another (and the permission bits).
pub fn copy<P: AsRef<Path>, Q: AsRef<Path>>(from: P, to: Q) -> io::Result<u64> {
    let from = from.as_ref();
    let to = to.as_ref();
    let mut reader = File::open(from)?;
    let meta = reader.metadata()?;
    if !meta.is_file() {
        return Err(io::const_io_error(
            io::ErrorKind::InvalidInput,
            "the source path is neither a regular file nor a symlink to a regular file",
        ));
    }
    let perm = meta.permissions();
    let mut writer = OpenOptions::new().write(true).create(true).truncate(true).open(to)?;
    let _ = writer.set_permissions(perm);
    io::copy(&mut reader, &mut writer)
}

/// Creates a new hard link on the filesystem.
pub fn hard_link<P: AsRef<Path>, Q: AsRef<Path>>(original: P, link: Q) -> io::Result<()> {
    let a = cpath(original.as_ref())?;
    let b = cpath(link.as_ref())?;
    sys::cvt(unsafe { sys::c_link(a.as_ptr(), b.as_ptr()) })?;
    Ok(())
}

/// Creates a new symbolic link on the filesystem (deprecated alias of `os::unix::fs::symlink`).
pub fn soft_link<P: AsRef<Path>, Q: AsRef<Path>>(original: P, link: Q) -> io::Result<()> {
    symlink_impl(original.as_ref(), link.as_ref())
}

pub(crate) fn symlink_impl(original: &Path, link: &Path) -> io::Result<()> {
    let a = cpath(original)?;
    let b = cpath(link)?;
    sys::cvt(unsafe { sys::c_symlink(a.as_ptr(), b.as_ptr()) })?;
    Ok(())
}

/// Reads a symbolic link, returning the file that the link points to.
pub fn read_link<P: AsRef<Path>>(path: P) -> io::Result<PathBuf> {
    let p = cpath(path.as_ref())?;
    let mut buf: Vec<u8> = Vec::with_capacity(256);
    loop {
        let cap = buf.capacity();
        let n = sys::cvt_isize(unsafe { sys::c_readlink(p.as_ptr(), buf.as_mut_ptr(), cap) })? as usize;
        if n < cap {
            unsafe { buf.set_len(n) };
            buf.shrink_to_fit();
            return Ok(PathBuf::from(OsString::from_vec(buf)));
        }
        buf.reserve(cap);
    }
}

/// Returns the canonical, absolute form of a path with all intermediate
/// components normalized and symbolic links resolved.
pub fn canonicalize<P: AsRef<Path>>(path: P) -> io::Result<PathBuf> {
    let p = cpath(path.as_ref())?;
    unsafe {
        let r = sys::c_realpath(p.as_ptr(), crate::ptr::null_mut());
        if r.is_null() {
            return Err(io::Error::last_os_error());
        }
        let v = sys::from_cstr(r);
        sys::c_free(r);
        Ok(PathBuf::from(OsString::from_vec(v)))
    }
}

/// Creates a new, empty directory at the provided path.
pub fn create_dir<P: AsRef<Path>>(path: P) -> io::Result<()> {
    DirBuilder::new().create(path.as_ref())
}

/// Recursively create a directory and all of its parent components if they are missing.
pub fn create_dir_all<P: AsRef<Path>>(path: P) -> io::Result<()> {
    DirBuilder::new().recursive(true).create(path.as_ref())
}

/// Removes an empty directory.
pub fn remove_dir<P: AsRef<Path>>(path: P) -> io::Result<()> {
    let p = cpath(path.as_ref())?;
    sys::cvt(unsafe { sys::c_rmdir(p.as_ptr()) })?;
    Ok(())
}

/// Removes a directory at this path, after removing all its contents.
pub fn remove_dir_all<P: AsRef<Path>>(path: P) -> io::Result<()> {
    let path = path.as_ref();
    let filetype = symlink_metadata(path)?.file_type();
    if filetype.is_symlink() { remove_file(path) } else { remove_dir_all_recursive(path) }
}

fn remove_dir_all_recursive(path: &Path) -> io::Result<()> {
    for child in read_dir(path)? {
        let child = child?;
        let child_path = child.path();
        let is_dir = match child.file_type() {
            Ok(t) => t.is_dir(),
            Err(e) => return Err(e),
        };
        if is_dir {
            match remove_dir_all_recursive(&child_path) {
                Ok(()) => {}
                Err(ref e) if e.kind() == io::ErrorKind::NotFound => {}
                Err(e) => return Err(e),
            }
        } else {
            match remove_file(&child_path) {
                Ok(()) => {}
                Err(ref e) if e.kind() == io::ErrorKind::NotFound => {}
                Err(e) => return Err(e),
            }
        }
    }
    remove_dir(path)
}

/// Changes the permissions found on a file or a directory.
pub fn set_permissions<P: AsRef<Path>>(path: P, perm: Permissions) -> io::Result<()> {
    let p = cpath(path.as_ref())?;
    sys::cvt_r(|| unsafe { sys::c_chmod(p.as_ptr(), perm.mode) })?;
    Ok(())
}

/// Returns `Ok(true)` if the path points at an existing entity.
pub fn exists<P: AsRef<Path>>(path: P) -> io::Result<bool> {
    match metadata(path) {
        Ok(_) => Ok(true),
        Err(ref e) if e.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(e),
    }
}
