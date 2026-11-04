//~ ERROR E0277
// `std::ptr::fn_addr_eq` takes fn pointers, not fn items.
fn a() {}

fn main() {
    println!("{}", std::ptr::fn_addr_eq(a, a));
}
