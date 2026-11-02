//! OS-specific functionality (Linux).

pub mod fd;
pub mod linux;
pub mod unix;

/// Platform-specific types, as defined by C.
pub mod raw {
    pub use crate::ffi::{
        c_char, c_double, c_float, c_int, c_long, c_longlong, c_schar, c_short, c_uchar, c_uint, c_ulong, c_ulonglong,
        c_ushort, c_void,
    };
}
