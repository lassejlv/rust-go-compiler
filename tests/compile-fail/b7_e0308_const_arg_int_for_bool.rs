//~ ERROR E0308
// An integer passed for a `bool` const parameter.
fn flag<const ON: bool>() -> bool {
    ON
}
fn main() {
    println!("{}", flag::<1>());
}
