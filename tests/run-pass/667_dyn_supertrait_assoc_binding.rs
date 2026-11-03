trait ByteSrc: Iterator<Item = u8> {}
struct Bs(u8);
impl Iterator for Bs {
    type Item = u8;
    fn next(&mut self) -> Option<u8> { if self.0 == 0 { None } else { self.0 -= 1; Some(self.0) } }
}
impl ByteSrc for Bs {}
fn main() {
    let mut b = Bs(4);
    let s: &mut dyn ByteSrc = &mut b;
    let v: Vec<u32> = s.map(|x| x as u32 * 10).collect();
    println!("{:?}", v);
}
