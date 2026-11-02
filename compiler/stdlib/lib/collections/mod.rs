//! Collection types.

pub mod binary_heap;
pub mod btree_map;
pub mod btree_set;
pub mod hash_map;
pub mod hash_set;
pub mod linked_list;
pub mod vec_deque;

/// `std::collections::btree` style aliases.
pub mod btree {
    pub use super::btree_map as map;
    pub use super::btree_set as set;
}

pub use self::binary_heap::BinaryHeap;
pub use self::btree_map::BTreeMap;
pub use self::btree_set::BTreeSet;
pub use self::hash_map::HashMap;
pub use self::hash_set::HashSet;
pub use self::linked_list::LinkedList;
pub use self::vec_deque::VecDeque;

pub use crate::ops::Bound;

use crate::fmt;

/// The error type for `try_reserve` methods.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct TryReserveError {
    kind: TryReserveErrorKind,
}

/// Details of the allocation that caused a `TryReserveError`.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum TryReserveErrorKind {
    /// Error due to the computed capacity exceeding the collection's maximum.
    CapacityOverflow,
    /// The memory allocator returned an error.
    AllocError,
}

impl TryReserveError {
    /// Details about the allocation that caused the error.
    pub fn kind(&self) -> TryReserveErrorKind {
        self.kind.clone()
    }
}

impl fmt::Display for TryReserveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("memory allocation failed")?;
        match self.kind {
            TryReserveErrorKind::CapacityOverflow => f.write_str(" because the computed capacity exceeded the collection's maximum"),
            TryReserveErrorKind::AllocError => f.write_str(" because the memory allocator returned an error"),
        }
    }
}

impl crate::error::Error for TryReserveError {}
