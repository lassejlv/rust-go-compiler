// Minimal std used to bring up the compiler pipeline.
pub mod marker {
    #[lang = "sized"] pub trait Sized {}
    #[lang = "copy"] pub trait Copy: Clone {}
    pub unsafe auto trait Send {}
    pub unsafe auto trait Sync {}
}
pub mod clone {
    #[lang = "clone"] pub trait Clone: Sized { fn clone(&self) -> Self; }
    impl Clone for i32 { fn clone(&self) -> i32 { *self } }
}
pub mod intrinsics {
    extern "rust-intrinsic" {
        pub fn write_stdout(p: *const u8, len: usize);
        pub fn str_ptr(p: *const str) -> *const u8;
        pub fn str_len(p: *const str) -> usize;
        pub fn panic_impl(msg: &str, file: &str, line: u32, col: u32) -> !;
        pub fn alloc(size: usize, align: usize) -> *mut u8;
        pub fn realloc(p: *mut u8, old: usize, align: usize, new: usize) -> *mut u8;
        pub fn ptr_add_mut<T>(p: *mut T, n: usize) -> *mut T;
        pub fn write<T>(p: *mut T, v: T);
        pub fn copy_nonoverlapping<T>(src: *const T, dst: *mut T, n: usize);
        pub fn str_from_raw_parts(p: *const u8, len: usize) -> *const str;
        pub fn discriminant_value<T>(v: &T) -> i64;
    }
}
pub mod ops {
    #[lang = "add"] pub trait Add<Rhs = Self> { type Output; fn add(self, rhs: Rhs) -> Self::Output; }
    #[lang = "fn_once"] pub trait FnOnce<Args> { type Output; fn call_once(self, args: Args) -> Self::Output; }
    #[lang = "fn_mut"] pub trait FnMut<Args>: FnOnce<Args> { fn call_mut(&mut self, args: Args) -> Self::Output; }
    #[lang = "fn"] pub trait Fn<Args>: FnMut<Args> { fn call(&self, args: Args) -> Self::Output; }
    #[lang = "deref"] pub trait Deref { type Target: ?Sized; fn deref(&self) -> &Self::Target; }
    #[lang = "drop"] pub trait Drop { fn drop(&mut self); }
    #[lang = "Range"] pub struct Range<Idx> { pub start: Idx, pub end: Idx }
    impl Iterator for Range<i32> {
        type Item = i32;
        fn next(&mut self) -> Option<i32> {
            if self.start < self.end { let v = self.start; self.start = self.start + 1; Some(v) } else { None }
        }
    }
    use crate::iter::Iterator;
    use crate::option::Option::{self, Some, None};
}
pub mod iter {
    #[lang = "iterator"] pub trait Iterator { type Item; fn next(&mut self) -> Option<Self::Item>; }
    #[lang = "into_iterator"] pub trait IntoIterator { type Item; type IntoIter: Iterator<Item = Self::Item>; fn into_iter(self) -> Self::IntoIter; }
    impl<I: Iterator> IntoIterator for I { type Item = I::Item; type IntoIter = I; fn into_iter(self) -> I { self } }
    use crate::option::Option;
}
pub mod option {
    #[lang = "Option"]
    pub enum Option<T> { #[lang = "None"] None, #[lang = "Some"] Some(T) }
    impl<T> Option<T> {
        pub fn unwrap(self) -> T { match self { Some(v) => v, None => crate::rt::panic_str("called `Option::unwrap()` on a `None` value", "", 0, 0) } }
        pub fn is_some(&self) -> bool { match self { Some(_) => true, None => false } }
    }
    use self::Option::{Some, None};
}
pub mod string {
    #[lang = "String"]
    pub struct String { ptr: *mut u8, len: usize, cap: usize }
    impl String {
        pub fn new() -> String { String { ptr: 0 as *mut u8, len: 0, cap: 0 } }
        pub fn from(s: &str) -> String { let mut r = String::new(); r.push_str(s); r }
        pub fn push_str(&mut self, s: &str) {
            let n = unsafe { crate::intrinsics::str_len(s) };
            if self.len + n > self.cap {
                let mut nc = self.cap * 2;
                if nc < self.len + n { nc = self.len + n + 8; }
                self.ptr = unsafe { crate::intrinsics::realloc(self.ptr, self.cap, 1, nc) };
                self.cap = nc;
            }
            unsafe { crate::intrinsics::copy_nonoverlapping(crate::intrinsics::str_ptr(s), crate::intrinsics::ptr_add_mut(self.ptr, self.len), n); }
            self.len = self.len + n;
        }
        pub fn as_str(&self) -> &str { unsafe { &*crate::intrinsics::str_from_raw_parts(self.ptr, self.len) } }
    }
}
pub mod fmt {
    use crate::string::String;
    pub type Result = crate::result::Result<(), Error>;
    pub struct Error;
    pub struct Arguments { s: String }
    impl Arguments {
        pub fn new(s: String) -> Arguments { Arguments { s: s } }
        pub fn new_const(s: &'static str) -> Arguments { Arguments { s: String::from(s) } }
    }
    pub struct Formatter { buf: String }
    impl Formatter {
        pub fn new() -> Formatter { Formatter { buf: String::new() } }
        pub fn write_str(&mut self, s: &str) -> Result { self.buf.push_str(s); Ok(()) }
        pub fn set_spec(&mut self, fill: char, align: u8, flags: u32, width: Option<usize>, prec: Option<usize>) {}
        pub fn into_string(self) -> String { self.buf }
    }
    pub trait Display { fn fmt(&self, f: &mut Formatter) -> Result; }
    pub trait Debug { fn fmt(&self, f: &mut Formatter) -> Result; }
    impl Display for str { fn fmt(&self, f: &mut Formatter) -> Result { f.write_str(self) } }
    impl<T: Display + ?Sized> Display for &T { fn fmt(&self, f: &mut Formatter) -> Result { (**self).fmt(f) } }
    impl Display for i32 {
        fn fmt(&self, f: &mut Formatter) -> Result {
            let mut buf = [0u8; 12];
            let mut n = *self;
            let neg = n < 0;
            let mut i = 12;
            if n == 0 { i = i - 1; buf[i] = 48; }
            while n != 0 { let d = n % 10; let d = if d < 0 { -d } else { d }; i = i - 1; buf[i] = (48 + d) as u8; n = n / 10; }
            if neg { i = i - 1; buf[i] = 45; }
            let s = unsafe { &*crate::intrinsics::str_from_raw_parts(&buf[i] as *const u8, 12 - i) };
            f.write_str(s)
        }
    }
    use crate::option::Option;
    use crate::result::Result::Ok;
}
pub mod result {
    #[lang = "Result"]
    pub enum Result<T, E> { #[lang = "Ok"] Ok(T), #[lang = "Err"] Err(E) }
}
pub mod io {
    pub fn _print(args: crate::fmt::Arguments) {
        let s = args.s.as_str();
        unsafe { crate::intrinsics::write_stdout(crate::intrinsics::str_ptr(s), crate::intrinsics::str_len(s)); }
    }
}
pub mod rt {
    pub fn panic_str(msg: &str, file: &str, line: u32, col: u32) -> ! { unsafe { crate::intrinsics::panic_impl(msg, file, line, col) } }
}
pub mod prelude {
    pub use crate::marker::{Sized, Copy, Send, Sync};
    pub use crate::clone::Clone;
    pub use crate::option::Option::{self, Some, None};
    pub use crate::result::Result::{self, Ok, Err};
    pub use crate::string::String;
    pub use crate::iter::{Iterator, IntoIterator};
    pub use crate::ops::{Fn, FnMut, FnOnce, Drop};
}
