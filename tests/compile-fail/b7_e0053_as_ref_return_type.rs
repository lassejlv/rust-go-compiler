//~ ERROR E0053
// `AsRef<str>::as_ref` returns `&str`, not `&String`.
struct Name(String);
impl AsRef<str> for Name {
    fn as_ref(&self) -> &String {
        &self.0
    }
}
fn main() {}
