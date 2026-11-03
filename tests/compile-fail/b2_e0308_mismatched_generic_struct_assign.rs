//~ ERROR E0308
struct B<T> { v: T }
fn main() {
    let mut b = B { v: 1u8 };
    b = B { v: "s" };
}
