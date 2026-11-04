//~ ERROR E0605
// Nor the ABI of a fn item.
extern "C" fn c_dec(x: i32) -> i32 { x - 1 }

fn main() {
    let r = c_dec as fn(i32) -> i32;
    println!("{}", r(1));
}
