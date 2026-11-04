//~ ERROR E0566
#[repr(u8, u32)]
enum Flag { On, Off }
fn main() { let _ = Flag::On; }
