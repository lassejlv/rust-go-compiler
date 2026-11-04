//~ ERROR E0433
// The crate root has no parent module.
mod util {
    pub fn f() {}
}
fn main() {
    super::util::f();
}
