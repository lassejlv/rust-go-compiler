//~ ERROR the `non_exhaustive` attribute cannot be used on functions
// `#[non_exhaustive]` applies to structs and enums.
#[non_exhaustive]
fn f() {}
fn main() {
    f();
}
