//! The `Drop` trait.

/// Custom code within the destructor.
#[lang = "drop"]
pub trait Drop {
    /// Executes the destructor for this type.
    fn drop(&mut self);
}
