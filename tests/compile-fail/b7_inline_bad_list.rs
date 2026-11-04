//~ ERROR valid forms for the attribute are
// Only `always` and `never` are accepted in the list.
#[inline(sometimes)]
fn f() {}
fn main() {
    f();
}
