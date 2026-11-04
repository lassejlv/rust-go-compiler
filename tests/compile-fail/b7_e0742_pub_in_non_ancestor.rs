//~ ERROR E0742
// `pub(in path)` must name an ancestor module.
mod a {
    pub(in crate::z) fn f() {}
}
mod z {}
fn main() {}
