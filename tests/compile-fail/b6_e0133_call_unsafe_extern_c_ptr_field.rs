//~ ERROR E0133
// Calling an `unsafe extern "C" fn` pointer stored in a struct field.
struct VTable {
    get: unsafe extern "C" fn(*const u32) -> u32,
}

unsafe extern "C" fn read_u32(p: *const u32) -> u32 { *p }

fn main() {
    let vt = VTable { get: read_u32 };
    let x = 5u32;
    println!("{}", (vt.get)(&x));
}
