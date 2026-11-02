//! The rustgo standard library.
//!
//! This crate is compiled as `std` before every user crate; `core` and
//! `alloc` are aliases of it. The module layout mirrors the real standard
//! library so that user programs resolve the same paths.
//!
//! NOTE: modules named like primitive types (`str`, `char`, `f64`, `i32`,
//! ...) live in this root module, so code in this file must not name
//! primitive types directly (they would resolve to those modules).

pub mod intrinsics;
pub mod marker;
pub mod ops;
pub mod cmp;
pub mod clone;
pub mod default;
pub mod convert;
pub mod option;
pub mod result;
pub mod mem;
pub mod ptr;
pub mod alloc;
pub mod boxed;
pub mod num;
pub mod char;
pub mod ascii;
pub mod str;
pub mod string;
pub mod slice;
pub mod array;
pub mod vec;
pub mod iter;
pub mod fmt;
pub mod hash;
pub mod borrow;
pub mod cell;
pub mod rc;
pub mod sync;
pub mod collections;
pub mod io;
pub mod fs;
pub mod path;
pub mod net;
pub mod process;
pub mod env;
pub mod any;
pub mod error;
pub mod hint;
pub mod rt;
pub mod tuple;
pub mod ffi;
pub mod os;
pub mod thread;
pub mod panic;
pub mod panicking;
pub mod time;
pub mod f32;
pub mod f64;
pub mod future;
pub mod task;
pub mod pin;
pub mod prelude;
#[doc(hidden)]
pub mod sys;

// Legacy integer modules: `std::i32::MAX`, `std::u64::MIN`, ...
macro_rules! __std_legacy_int_modules {
    ($($t:ident)*) => {
        $(
            pub mod $t {
                //! Legacy constants for this integer type.
                pub const MIN: $t = $t::MIN;
                pub const MAX: $t = $t::MAX;
            }
        )*
    };
}

__std_legacy_int_modules! { i8 i16 i32 i64 i128 isize u8 u16 u32 u64 u128 usize }
