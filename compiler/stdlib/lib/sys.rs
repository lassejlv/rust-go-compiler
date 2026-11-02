//! Thin bindings to the C library (Linux x86-64, glibc) used by the
//! platform-dependent parts of the library (`fs`, `process`, `net`, ...).
//!
//! Every foreign function has a private Rust name (`c_*`) bound to the real
//! symbol with `#[link_name]`, so that it never clashes with declarations of
//! the same C function in user code.

#![allow(non_camel_case_types)]

use crate::io;

pub type c_int = i32;
pub type c_uint = u32;
pub type c_long = i64;
pub type c_ulong = u64;
pub type size_t = usize;
pub type ssize_t = isize;
pub type off_t = i64;
pub type mode_t = u32;
pub type pid_t = i32;
pub type uid_t = u32;
pub type gid_t = u32;
pub type socklen_t = u32;

/// `struct stat` (x86-64).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct stat_t {
    pub st_dev: u64,
    pub st_ino: u64,
    pub st_nlink: u64,
    pub st_mode: u32,
    pub st_uid: u32,
    pub st_gid: u32,
    pub __pad0: i32,
    pub st_rdev: u64,
    pub st_size: i64,
    pub st_blksize: i64,
    pub st_blocks: i64,
    pub st_atime: i64,
    pub st_atime_nsec: i64,
    pub st_mtime: i64,
    pub st_mtime_nsec: i64,
    pub st_ctime: i64,
    pub st_ctime_nsec: i64,
    pub __unused: [i64; 3],
}

impl stat_t {
    pub fn zeroed() -> stat_t {
        unsafe { crate::mem::zeroed() }
    }
}

/// `struct dirent` (x86-64, glibc).
#[repr(C)]
pub struct dirent_t {
    pub d_ino: u64,
    pub d_off: i64,
    pub d_reclen: u16,
    pub d_type: u8,
    pub d_name: [u8; 256],
}

/// `struct timespec`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct timespec_t {
    pub tv_sec: i64,
    pub tv_nsec: i64,
}

/// `struct iovec`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct iovec_t {
    pub iov_base: *mut u8,
    pub iov_len: usize,
}

/// `struct pollfd`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct pollfd_t {
    pub fd: c_int,
    pub events: i16,
    pub revents: i16,
}

/// `struct addrinfo`.
#[repr(C)]
pub struct addrinfo_t {
    pub ai_flags: c_int,
    pub ai_family: c_int,
    pub ai_socktype: c_int,
    pub ai_protocol: c_int,
    pub ai_addrlen: socklen_t,
    pub ai_addr: *mut u8,
    pub ai_canonname: *mut u8,
    pub ai_next: *mut addrinfo_t,
}

/// `struct linger`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct linger_t {
    pub l_onoff: c_int,
    pub l_linger: c_int,
}

/// `struct timeval`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct timeval_t {
    pub tv_sec: i64,
    pub tv_usec: i64,
}

// open(2) flags
pub const O_RDONLY: c_int = 0;
pub const O_WRONLY: c_int = 1;
pub const O_RDWR: c_int = 2;
pub const O_ACCMODE: c_int = 3;
pub const O_CREAT: c_int = 0o100;
pub const O_EXCL: c_int = 0o200;
pub const O_NOCTTY: c_int = 0o400;
pub const O_TRUNC: c_int = 0o1000;
pub const O_APPEND: c_int = 0o2000;
pub const O_NONBLOCK: c_int = 0o4000;
pub const O_DIRECTORY: c_int = 0o200000;
pub const O_NOFOLLOW: c_int = 0o400000;
pub const O_CLOEXEC: c_int = 0o2000000;

// file types (st_mode)
pub const S_IFMT: u32 = 0o170000;
pub const S_IFSOCK: u32 = 0o140000;
pub const S_IFLNK: u32 = 0o120000;
pub const S_IFREG: u32 = 0o100000;
pub const S_IFBLK: u32 = 0o060000;
pub const S_IFDIR: u32 = 0o040000;
pub const S_IFCHR: u32 = 0o020000;
pub const S_IFIFO: u32 = 0o010000;

// dirent d_type
pub const DT_UNKNOWN: u8 = 0;
pub const DT_FIFO: u8 = 1;
pub const DT_CHR: u8 = 2;
pub const DT_DIR: u8 = 4;
pub const DT_BLK: u8 = 6;
pub const DT_REG: u8 = 8;
pub const DT_LNK: u8 = 10;
pub const DT_SOCK: u8 = 12;

// lseek whence
pub const SEEK_SET: c_int = 0;
pub const SEEK_CUR: c_int = 1;
pub const SEEK_END: c_int = 2;

// fcntl
pub const F_DUPFD: c_int = 0;
pub const F_GETFD: c_int = 1;
pub const F_SETFD: c_int = 2;
pub const F_GETFL: c_int = 3;
pub const F_SETFL: c_int = 4;
pub const F_DUPFD_CLOEXEC: c_int = 1030;
pub const FD_CLOEXEC: c_int = 1;

pub const AT_FDCWD: c_int = -100;
pub const AT_SYMLINK_NOFOLLOW: c_int = 0x100;
pub const AT_REMOVEDIR: c_int = 0x200;
pub const UTIME_OMIT: i64 = (1 << 30) - 2;

// wait
pub const WNOHANG: c_int = 1;

// signals
pub const SIGKILL: c_int = 9;
pub const SIGPIPE: c_int = 13;

// sockets
pub const AF_UNIX: c_int = 1;
pub const AF_INET: c_int = 2;
pub const AF_INET6: c_int = 10;
pub const SOCK_STREAM: c_int = 1;
pub const SOCK_DGRAM: c_int = 2;
pub const SOCK_NONBLOCK: c_int = 0o4000;
pub const SOCK_CLOEXEC: c_int = 0o2000000;
pub const SOL_SOCKET: c_int = 1;
pub const SO_REUSEADDR: c_int = 2;
pub const SO_TYPE: c_int = 3;
pub const SO_ERROR: c_int = 4;
pub const SO_BROADCAST: c_int = 6;
pub const SO_SNDBUF: c_int = 7;
pub const SO_RCVBUF: c_int = 8;
pub const SO_LINGER: c_int = 13;
pub const SO_PEERCRED: c_int = 17;
pub const SO_RCVTIMEO: c_int = 20;
pub const SO_SNDTIMEO: c_int = 21;
pub const IPPROTO_IP: c_int = 0;
pub const IPPROTO_TCP: c_int = 6;
pub const IPPROTO_IPV6: c_int = 41;
pub const IP_TTL: c_int = 2;
pub const IP_MULTICAST_TTL: c_int = 33;
pub const IP_MULTICAST_LOOP: c_int = 34;
pub const IPV6_V6ONLY: c_int = 26;
pub const TCP_NODELAY: c_int = 1;
pub const MSG_PEEK: c_int = 2;
pub const MSG_NOSIGNAL: c_int = 0x4000;
pub const SHUT_RD: c_int = 0;
pub const SHUT_WR: c_int = 1;
pub const SHUT_RDWR: c_int = 2;
pub const POLLIN: i16 = 1;
pub const POLLOUT: i16 = 4;

// errno values
pub const EPERM: i32 = 1;
pub const ENOENT: i32 = 2;
pub const ESRCH: i32 = 3;
pub const EINTR: i32 = 4;
pub const EIO: i32 = 5;
pub const E2BIG: i32 = 7;
pub const EBADF: i32 = 9;
pub const ECHILD: i32 = 10;
pub const EAGAIN: i32 = 11;
pub const ENOMEM: i32 = 12;
pub const EACCES: i32 = 13;
pub const EBUSY: i32 = 16;
pub const EEXIST: i32 = 17;
pub const EXDEV: i32 = 18;
pub const ENOTDIR: i32 = 20;
pub const EISDIR: i32 = 21;
pub const EINVAL: i32 = 22;
pub const ETXTBSY: i32 = 26;
pub const EFBIG: i32 = 27;
pub const ENOSPC: i32 = 28;
pub const ESPIPE: i32 = 29;
pub const EROFS: i32 = 30;
pub const EMLINK: i32 = 31;
pub const EPIPE: i32 = 32;
pub const EDEADLK: i32 = 35;
pub const ENAMETOOLONG: i32 = 36;
pub const ENOSYS: i32 = 38;
pub const ENOTEMPTY: i32 = 39;
pub const ELOOP: i32 = 40;
pub const ENETDOWN: i32 = 100;
pub const ENETUNREACH: i32 = 101;
pub const ECONNABORTED: i32 = 103;
pub const ECONNRESET: i32 = 104;
pub const ENOTCONN: i32 = 107;
pub const ETIMEDOUT: i32 = 110;
pub const ECONNREFUSED: i32 = 111;
pub const EHOSTUNREACH: i32 = 113;
pub const EADDRINUSE: i32 = 98;
pub const EADDRNOTAVAIL: i32 = 99;
pub const EINPROGRESS: i32 = 115;
pub const ESTALE: i32 = 116;
pub const EDQUOT: i32 = 122;

extern "C" {
    #[link_name = "__errno_location"]
    pub fn c_errno_location() -> *mut c_int;
    #[link_name = "__xpg_strerror_r"]
    pub fn c_strerror_r(errnum: c_int, buf: *mut u8, n: size_t) -> c_int;

    // ---- files and descriptors
    #[link_name = "open"]
    pub fn c_open(path: *const u8, flags: c_int, ...) -> c_int;
    #[link_name = "openat"]
    pub fn c_openat(dirfd: c_int, path: *const u8, flags: c_int, ...) -> c_int;
    #[link_name = "close"]
    pub fn c_close(fd: c_int) -> c_int;
    #[link_name = "read"]
    pub fn c_read(fd: c_int, buf: *mut u8, n: size_t) -> ssize_t;
    #[link_name = "write"]
    pub fn c_write(fd: c_int, buf: *const u8, n: size_t) -> ssize_t;
    #[link_name = "readv"]
    pub fn c_readv(fd: c_int, iov: *const iovec_t, n: c_int) -> ssize_t;
    #[link_name = "writev"]
    pub fn c_writev(fd: c_int, iov: *const iovec_t, n: c_int) -> ssize_t;
    #[link_name = "pread64"]
    pub fn c_pread(fd: c_int, buf: *mut u8, n: size_t, off: off_t) -> ssize_t;
    #[link_name = "pwrite64"]
    pub fn c_pwrite(fd: c_int, buf: *const u8, n: size_t, off: off_t) -> ssize_t;
    #[link_name = "lseek64"]
    pub fn c_lseek(fd: c_int, off: off_t, whence: c_int) -> off_t;
    #[link_name = "fsync"]
    pub fn c_fsync(fd: c_int) -> c_int;
    #[link_name = "fdatasync"]
    pub fn c_fdatasync(fd: c_int) -> c_int;
    #[link_name = "ftruncate64"]
    pub fn c_ftruncate(fd: c_int, len: off_t) -> c_int;
    #[link_name = "fstat64"]
    pub fn c_fstat(fd: c_int, buf: *mut stat_t) -> c_int;
    #[link_name = "stat64"]
    pub fn c_stat(path: *const u8, buf: *mut stat_t) -> c_int;
    #[link_name = "lstat64"]
    pub fn c_lstat(path: *const u8, buf: *mut stat_t) -> c_int;
    #[link_name = "fchmod"]
    pub fn c_fchmod(fd: c_int, mode: mode_t) -> c_int;
    #[link_name = "chmod"]
    pub fn c_chmod(path: *const u8, mode: mode_t) -> c_int;
    #[link_name = "chown"]
    pub fn c_chown(path: *const u8, uid: uid_t, gid: gid_t) -> c_int;
    #[link_name = "fchown"]
    pub fn c_fchown(fd: c_int, uid: uid_t, gid: gid_t) -> c_int;
    #[link_name = "lchown"]
    pub fn c_lchown(path: *const u8, uid: uid_t, gid: gid_t) -> c_int;
    #[link_name = "futimens"]
    pub fn c_futimens(fd: c_int, times: *const timespec_t) -> c_int;
    #[link_name = "mkdir"]
    pub fn c_mkdir(path: *const u8, mode: mode_t) -> c_int;
    #[link_name = "rmdir"]
    pub fn c_rmdir(path: *const u8) -> c_int;
    #[link_name = "unlink"]
    pub fn c_unlink(path: *const u8) -> c_int;
    #[link_name = "unlinkat"]
    pub fn c_unlinkat(dirfd: c_int, path: *const u8, flags: c_int) -> c_int;
    #[link_name = "rename"]
    pub fn c_rename(old: *const u8, new: *const u8) -> c_int;
    #[link_name = "link"]
    pub fn c_link(old: *const u8, new: *const u8) -> c_int;
    #[link_name = "symlink"]
    pub fn c_symlink(target: *const u8, path: *const u8) -> c_int;
    #[link_name = "readlink"]
    pub fn c_readlink(path: *const u8, buf: *mut u8, n: size_t) -> ssize_t;
    #[link_name = "realpath"]
    pub fn c_realpath(path: *const u8, resolved: *mut u8) -> *mut u8;
    #[link_name = "access"]
    pub fn c_access(path: *const u8, mode: c_int) -> c_int;
    #[link_name = "getcwd"]
    pub fn c_getcwd(buf: *mut u8, n: size_t) -> *mut u8;
    #[link_name = "chdir"]
    pub fn c_chdir(path: *const u8) -> c_int;
    #[link_name = "opendir"]
    pub fn c_opendir(path: *const u8) -> *mut u8;
    #[link_name = "fdopendir"]
    pub fn c_fdopendir(fd: c_int) -> *mut u8;
    #[link_name = "readdir64"]
    pub fn c_readdir(dir: *mut u8) -> *mut dirent_t;
    #[link_name = "closedir"]
    pub fn c_closedir(dir: *mut u8) -> c_int;
    #[link_name = "dup"]
    pub fn c_dup(fd: c_int) -> c_int;
    #[link_name = "dup2"]
    pub fn c_dup2(fd: c_int, fd2: c_int) -> c_int;
    #[link_name = "fcntl"]
    pub fn c_fcntl(fd: c_int, cmd: c_int, ...) -> c_int;
    #[link_name = "ioctl"]
    pub fn c_ioctl(fd: c_int, req: c_ulong, ...) -> c_int;
    #[link_name = "pipe2"]
    pub fn c_pipe2(fds: *mut c_int, flags: c_int) -> c_int;
    #[link_name = "isatty"]
    pub fn c_isatty(fd: c_int) -> c_int;
    #[link_name = "poll"]
    pub fn c_poll(fds: *mut pollfd_t, n: u64, timeout: c_int) -> c_int;
    #[link_name = "umask"]
    pub fn c_umask(mask: mode_t) -> mode_t;

    // ---- processes
    #[link_name = "fork"]
    pub fn c_fork() -> pid_t;
    #[link_name = "execvp"]
    pub fn c_execvp(file: *const u8, argv: *const *const u8) -> c_int;
    #[link_name = "execvpe"]
    pub fn c_execvpe(file: *const u8, argv: *const *const u8, envp: *const *const u8) -> c_int;
    #[link_name = "_exit"]
    pub fn c_exit_now(code: c_int) -> !;
    #[link_name = "waitpid"]
    pub fn c_waitpid(pid: pid_t, status: *mut c_int, options: c_int) -> pid_t;
    #[link_name = "kill"]
    pub fn c_kill(pid: pid_t, sig: c_int) -> c_int;
    #[link_name = "getppid"]
    pub fn c_getppid() -> pid_t;
    #[link_name = "getuid"]
    pub fn c_getuid() -> uid_t;
    #[link_name = "geteuid"]
    pub fn c_geteuid() -> uid_t;
    #[link_name = "getgid"]
    pub fn c_getgid() -> gid_t;
    #[link_name = "setenv"]
    pub fn c_setenv(name: *const u8, value: *const u8, overwrite: c_int) -> c_int;
    #[link_name = "unsetenv"]
    pub fn c_unsetenv(name: *const u8) -> c_int;
    #[link_name = "signal"]
    pub fn c_signal(sig: c_int, handler: usize) -> usize;
    #[link_name = "setsid"]
    pub fn c_setsid() -> pid_t;
    #[link_name = "setpgid"]
    pub fn c_setpgid(pid: pid_t, pgid: pid_t) -> c_int;
    #[link_name = "setuid"]
    pub fn c_setuid(uid: uid_t) -> c_int;
    #[link_name = "setgid"]
    pub fn c_setgid(gid: gid_t) -> c_int;

    // ---- sockets
    #[link_name = "socket"]
    pub fn c_socket(domain: c_int, ty: c_int, proto: c_int) -> c_int;
    #[link_name = "socketpair"]
    pub fn c_socketpair(domain: c_int, ty: c_int, proto: c_int, fds: *mut c_int) -> c_int;
    #[link_name = "bind"]
    pub fn c_bind(fd: c_int, addr: *const u8, len: socklen_t) -> c_int;
    #[link_name = "listen"]
    pub fn c_listen(fd: c_int, backlog: c_int) -> c_int;
    #[link_name = "accept4"]
    pub fn c_accept4(fd: c_int, addr: *mut u8, len: *mut socklen_t, flags: c_int) -> c_int;
    #[link_name = "connect"]
    pub fn c_connect(fd: c_int, addr: *const u8, len: socklen_t) -> c_int;
    #[link_name = "send"]
    pub fn c_send(fd: c_int, buf: *const u8, n: size_t, flags: c_int) -> ssize_t;
    #[link_name = "recv"]
    pub fn c_recv(fd: c_int, buf: *mut u8, n: size_t, flags: c_int) -> ssize_t;
    #[link_name = "sendto"]
    pub fn c_sendto(fd: c_int, buf: *const u8, n: size_t, flags: c_int, addr: *const u8, len: socklen_t) -> ssize_t;
    #[link_name = "recvfrom"]
    pub fn c_recvfrom(fd: c_int, buf: *mut u8, n: size_t, flags: c_int, addr: *mut u8, len: *mut socklen_t) -> ssize_t;
    #[link_name = "shutdown"]
    pub fn c_shutdown(fd: c_int, how: c_int) -> c_int;
    #[link_name = "getsockname"]
    pub fn c_getsockname(fd: c_int, addr: *mut u8, len: *mut socklen_t) -> c_int;
    #[link_name = "getpeername"]
    pub fn c_getpeername(fd: c_int, addr: *mut u8, len: *mut socklen_t) -> c_int;
    #[link_name = "setsockopt"]
    pub fn c_setsockopt(fd: c_int, level: c_int, name: c_int, val: *const u8, len: socklen_t) -> c_int;
    #[link_name = "getsockopt"]
    pub fn c_getsockopt(fd: c_int, level: c_int, name: c_int, val: *mut u8, len: *mut socklen_t) -> c_int;
    #[link_name = "getaddrinfo"]
    pub fn c_getaddrinfo(node: *const u8, service: *const u8, hints: *const addrinfo_t, res: *mut *mut addrinfo_t) -> c_int;
    #[link_name = "freeaddrinfo"]
    pub fn c_freeaddrinfo(res: *mut addrinfo_t);
    #[link_name = "gai_strerror"]
    pub fn c_gai_strerror(code: c_int) -> *const u8;

    // ---- memory
    #[link_name = "malloc"]
    pub fn c_malloc(n: size_t) -> *mut u8;
    #[link_name = "calloc"]
    pub fn c_calloc(n: size_t, size: size_t) -> *mut u8;
    #[link_name = "free"]
    pub fn c_free(p: *mut u8);
    #[link_name = "posix_memalign"]
    pub fn c_posix_memalign(out: *mut *mut u8, align: size_t, size: size_t) -> c_int;
    #[link_name = "realloc"]
    pub fn c_realloc(p: *mut u8, n: size_t) -> *mut u8;
    #[link_name = "fflush"]
    pub fn c_fflush(stream: *mut u8) -> c_int;
}

/// The calling thread's current `errno`.
#[inline]
pub fn errno() -> i32 {
    unsafe { *c_errno_location() }
}

/// Sets the calling thread's `errno`.
#[inline]
pub fn set_errno(v: i32) {
    unsafe { *c_errno_location() = v }
}

/// The C library's message for an error number (`strerror`).
pub fn error_string(errno: i32) -> String {
    let mut buf = [0u8; 128];
    unsafe {
        if c_strerror_r(errno, buf.as_mut_ptr(), buf.len()) < 0 {
            return String::from("strerror_r failure");
        }
    }
    let mut n = 0;
    while n < buf.len() && buf[n] != 0 {
        n += 1;
    }
    String::from_utf8_lossy(&buf[..n]).into_owned()
}

/// Converts a C `-1` result into the current `errno` error.
#[inline]
pub fn cvt(r: c_int) -> io::Result<c_int> {
    if r == -1 { Err(io::Error::last_os_error()) } else { Ok(r) }
}

/// Like `cvt` for `ssize_t` results.
#[inline]
pub fn cvt_isize(r: isize) -> io::Result<isize> {
    if r == -1 { Err(io::Error::last_os_error()) } else { Ok(r) }
}

/// Like `cvt` for `off_t` results.
#[inline]
pub fn cvt_i64(r: i64) -> io::Result<i64> {
    if r == -1 { Err(io::Error::last_os_error()) } else { Ok(r) }
}

/// Retries `f` while it fails with `EINTR`.
pub fn cvt_r<F: FnMut() -> c_int>(mut f: F) -> io::Result<c_int> {
    loop {
        let r = f();
        if r == -1 {
            let e = errno();
            if e == EINTR {
                continue;
            }
            return Err(io::Error::from_raw_os_error(e));
        }
        return Ok(r);
    }
}

/// Copies `bytes` into a NUL-terminated buffer (fails on interior NULs
/// like the real library: "file name contained an unexpected NUL byte").
pub fn cstr(bytes: &[u8]) -> io::Result<Vec<u8>> {
    if bytes.contains(&0) {
        return Err(io::const_io_error(io::ErrorKind::InvalidInput, "file name contained an unexpected NUL byte"));
    }
    let mut v = Vec::with_capacity(bytes.len() + 1);
    v.extend_from_slice(bytes);
    v.push(0);
    Ok(v)
}

/// Copies a NUL-terminated C string into a byte vector.
pub unsafe fn from_cstr(p: *const u8) -> Vec<u8> {
    let n = crate::intrinsics::cstr_len(p);
    crate::slice::from_raw_parts(p, n).to_vec()
}

/// Flushes the buffered standard output (before a fork or a raw write).
pub fn flush_stdout() {
    unsafe { crate::intrinsics::flush_stdout() }
}

/// Reads from a descriptor, retrying on `EINTR`.
pub fn read_fd(fd: c_int, buf: &mut [u8]) -> io::Result<usize> {
    let len = if buf.len() > isize::MAX as usize { isize::MAX as usize } else { buf.len() };
    loop {
        let r = unsafe { c_read(fd, buf.as_mut_ptr(), len) };
        if r < 0 {
            let e = errno();
            if e == EINTR {
                continue;
            }
            return Err(io::Error::from_raw_os_error(e));
        }
        return Ok(r as usize);
    }
}

/// Writes to a descriptor, retrying on `EINTR`.
pub fn write_fd(fd: c_int, buf: &[u8]) -> io::Result<usize> {
    let len = if buf.len() > isize::MAX as usize { isize::MAX as usize } else { buf.len() };
    if fd == 1 {
        flush_stdout();
    }
    loop {
        let r = unsafe { c_write(fd, buf.as_ptr(), len) };
        if r < 0 {
            let e = errno();
            if e == EINTR {
                continue;
            }
            return Err(io::Error::from_raw_os_error(e));
        }
        return Ok(r as usize);
    }
}

/// Sets or clears `O_NONBLOCK` on a descriptor.
pub fn set_nonblocking(fd: c_int, nonblocking: bool) -> io::Result<()> {
    unsafe {
        let flags = cvt(c_fcntl(fd, F_GETFL))?;
        let new = if nonblocking { flags | O_NONBLOCK } else { flags & !O_NONBLOCK };
        if new != flags {
            cvt(c_fcntl(fd, F_SETFL, new))?;
        }
    }
    Ok(())
}

/// Sets or clears `FD_CLOEXEC` on a descriptor.
pub fn set_cloexec(fd: c_int, cloexec: bool) -> io::Result<()> {
    unsafe {
        let flags = cvt(c_fcntl(fd, F_GETFD))?;
        let new = if cloexec { flags | FD_CLOEXEC } else { flags & !FD_CLOEXEC };
        if new != flags {
            cvt(c_fcntl(fd, F_SETFD, new))?;
        }
    }
    Ok(())
}

/// Duplicates a descriptor (the copy has `FD_CLOEXEC` set).
pub fn dup_cloexec(fd: c_int) -> io::Result<c_int> {
    unsafe { cvt(c_fcntl(fd, F_DUPFD_CLOEXEC, 3)) }
}
