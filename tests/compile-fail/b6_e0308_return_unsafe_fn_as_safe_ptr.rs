//~ ERROR E0308
// Returning an unsafe fn item where a safe fn pointer is declared.
unsafe fn raw_get(p: *const u8) -> u8 { *p }
fn safe_get(p: *const u8) -> u8 { if p.is_null() { 0 } else { 1 } }

fn pick(fast: bool) -> fn(*const u8) -> u8 {
    if fast { raw_get } else { safe_get }
}

fn main() {
    let b = 7u8;
    println!("{}", pick(true)(&b));
}
