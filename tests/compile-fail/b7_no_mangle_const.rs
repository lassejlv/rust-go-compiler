//~ ERROR const items should never be `#[no_mangle]`
// no_mangle_const_items is deny-by-default.
#[no_mangle]
pub const LIMIT: u32 = 10;
fn main() {
    println!("{}", LIMIT);
}
