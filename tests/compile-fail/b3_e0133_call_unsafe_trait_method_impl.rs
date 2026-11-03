//~ ERROR E0133
struct Raw(*const u8);
impl Raw { unsafe fn read(&self) -> u8 { *self.0 } }
fn main() { let x = 1u8; let r = Raw(&x); println!("{}", r.read()); }
