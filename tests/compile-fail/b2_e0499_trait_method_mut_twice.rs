//~ ERROR E0499
trait Store { fn slot(&mut self) -> &mut u32; }
struct S { x: u32 }
impl Store for S { fn slot(&mut self) -> &mut u32 { &mut self.x } }
fn bump<T: Store>(t: &mut T) {
    let a = t.slot();
    let b = t.slot();
    *a += *b;
}
fn main() { let mut s = S { x: 1 }; bump(&mut s); }
