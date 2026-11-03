//~ ERROR E0599
struct Cfg;
impl Cfg { const A: u32 = 1; }
fn main() { println!("{}", Cfg::B); }
