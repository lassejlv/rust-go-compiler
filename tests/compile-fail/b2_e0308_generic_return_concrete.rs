//~ ERROR E0308
fn make<T: Default>() -> T {
    0
}
fn main() { let _: i32 = make(); }
