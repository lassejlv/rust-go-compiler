//~ ERROR valid forms for the attribute are
// `#[inline = ".."]` is not a valid form.
#[inline = "always"]
fn f() {}
fn main() {
    f();
}
