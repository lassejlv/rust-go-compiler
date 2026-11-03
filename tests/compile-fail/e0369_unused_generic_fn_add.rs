//~ ERROR E0369
// Never instantiated: still checked (T has no Add bound).
fn double<T: Copy>(x: T) -> T {
    x + x
}

fn main() {
    println!("ok");
}
