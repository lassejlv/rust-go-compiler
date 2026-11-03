//~ ERROR E0133
static mut HITS: u32 = 0;
fn hit() { HITS += 1; }
fn main() { hit(); }
