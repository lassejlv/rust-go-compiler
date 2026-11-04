//~ ERROR E0117
// `Option<Local>` is not a local type either.
struct Local;
impl<T> From<Option<T>> for Vec<Local> {
    fn from(_: Option<T>) -> Self {
        Vec::new()
    }
}
fn main() {}
