//~ ERROR E0308
// Only non-capturing closures coerce to fn pointers.
fn main() {
    let mut n = 0;
    let f: fn() = || n += 1;
    f();
}
