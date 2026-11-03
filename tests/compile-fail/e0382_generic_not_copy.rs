//~ ERROR E0382
fn dup<T>(x: T) -> (T, T) {
    (x, x)
}

fn main() {
    let (a, b) = dup(1);
    println!("{} {}", a, b);
}
