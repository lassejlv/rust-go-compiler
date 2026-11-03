//~ ERROR E0596
struct Buf { data: Vec<u8> }
fn fill(b: &Buf) { let d = &mut b.data; d.push(0); }
fn main() { fill(&Buf { data: vec![] }); }
