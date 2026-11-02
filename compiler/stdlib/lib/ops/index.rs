//! `Index` and `IndexMut`.

/// Used for indexing operations (`container[index]`) in immutable contexts.
#[lang = "index"]
pub trait Index<Idx: ?Sized> {
    /// The returned type after indexing.
    type Output: ?Sized;
    /// Performs the indexing (`container[index]`) operation.
    fn index(&self, index: Idx) -> &Self::Output;
}

/// Used for indexing operations (`container[index]`) in mutable contexts.
#[lang = "index_mut"]
pub trait IndexMut<Idx: ?Sized>: Index<Idx> {
    /// Performs the mutable indexing (`container[index]`) operation.
    fn index_mut(&mut self, index: Idx) -> &mut Self::Output;
}
