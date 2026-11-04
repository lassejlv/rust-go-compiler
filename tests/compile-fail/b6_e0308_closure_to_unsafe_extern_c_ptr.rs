//~ ERROR E0308
// Not even to `unsafe extern "C"` ones.
fn register(_cb: unsafe extern "C" fn(u32) -> u32) {}

fn main() {
    register(|x| x * 2);
}
