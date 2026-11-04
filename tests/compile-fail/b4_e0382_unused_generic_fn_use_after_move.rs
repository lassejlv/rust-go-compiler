//~ ERROR E0382
// Generic functions are borrow-checked even without an instance.
fn pair<T>(x: T) -> (T, T) {
    (x, x)
}
fn main() {}
