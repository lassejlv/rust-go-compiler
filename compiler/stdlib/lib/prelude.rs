//! The standard library prelude: names that are in scope in every module.

/// The first version of the prelude.
pub mod v1 {
    pub use crate::borrow::ToOwned;
    pub use crate::boxed::Box;
    pub use crate::clone::Clone;
    pub use crate::cmp::{Eq, Ord, PartialEq, PartialOrd};
    pub use crate::convert::{AsMut, AsRef, From, Into};
    pub use crate::default::Default;
    pub use crate::iter::{DoubleEndedIterator, ExactSizeIterator, Extend, IntoIterator, Iterator};
    pub use crate::marker::{Copy, Send, Sized, Sync, Unpin};
    pub use crate::mem::drop;
    pub use crate::mem::{align_of, align_of_val, size_of, size_of_val};
    pub use crate::ops::{Drop, Fn, FnMut, FnOnce};
    pub use crate::future::{AsyncFn, AsyncFnMut, AsyncFnOnce};
    pub use crate::option::Option;
    pub use crate::option::Option::{None, Some};
    pub use crate::result::Result;
    pub use crate::result::Result::{Err, Ok};
    pub use crate::string::{String, ToString};
    pub use crate::vec::Vec;
}

/// The 2015 version of the prelude.
pub mod rust_2015 {
    pub use super::v1::*;
}

/// The 2018 version of the prelude.
pub mod rust_2018 {
    pub use super::v1::*;
}

/// The 2021 version of the prelude.
pub mod rust_2021 {
    pub use super::v1::*;
    pub use crate::convert::{TryFrom, TryInto};
    pub use crate::iter::FromIterator;
}

/// The 2024 version of the prelude.
pub mod rust_2024 {
    pub use super::rust_2021::*;
    pub use crate::future::{Future, IntoFuture};
}

pub use self::rust_2021::*;
