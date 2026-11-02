//! Composable external iteration.

mod adapters;
mod range;
mod sources;
mod traits;

pub use self::adapters::{
    Chain, Cloned, Copied, Cycle, Enumerate, Filter, FilterMap, FlatMap, Flatten, Fuse, Inspect, Map, MapWhile, Peekable,
    Rev, Scan, Skip, SkipWhile, StepBy, Take, TakeWhile, Zip,
};
pub use self::range::Step;
pub use self::sources::{
    empty, from_fn, once, once_with, repeat, repeat_n, repeat_with, successors, zip, Empty, FromFn, Once, OnceWith, Repeat,
    RepeatN, RepeatWith, Successors,
};
pub use self::traits::{
    DoubleEndedIterator, ExactSizeIterator, Extend, FromIterator, FusedIterator, IntoIterator, Iterator, Product, Sum,
    TrustedLen,
};
