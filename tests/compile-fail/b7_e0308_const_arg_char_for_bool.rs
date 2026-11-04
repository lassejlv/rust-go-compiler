//~ ERROR E0308
// A char for a bool const parameter of a struct.
struct Switch<const ON: bool>;
fn main() {
    let _s: Switch<'y'> = Switch;
}
