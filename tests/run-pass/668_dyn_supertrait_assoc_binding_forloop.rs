trait ByteSrc: Iterator<Item = u8> {}
struct Bs(u8);
impl Iterator for Bs { type Item = u8; fn next(&mut self) -> Option<u8> { if self.0 == 0 { None } else { self.0 -= 1; Some(self.0) } } }
impl ByteSrc for Bs {}
fn main() {
    let b: Box<dyn ByteSrc> = Box::new(Bs(3));
    for x in b { println!("{}", x + 1); }
}
