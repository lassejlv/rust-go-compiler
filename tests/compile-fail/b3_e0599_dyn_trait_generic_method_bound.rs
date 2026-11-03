//~ ERROR E0599
trait Store { fn get(&self) -> i32; }
struct Mem;
impl Store for Mem { fn get(&self) -> i32 { 1 } }
fn main() {
    let s: Box<dyn Store> = Box::new(Mem);
    s.put(2);
}
