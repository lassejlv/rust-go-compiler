#[derive(Clone, Copy, Debug)]
struct Code { op: u8, bits: u8, val: u16 }
const fn code(op: u8, bits: u8, val: u16) -> Code { Code { op, bits, val } }
const LENFIX: [Code; 4] = [code(96, 7, 0), code(0, 8, 80), code(0, 8, 16), code(20, 8, 115)];
struct S { which: u8, own: [Code; 2] }
impl S {
    fn get(&self, i: usize) -> Code {
        let t = match self.which { 0 => &LENFIX[..], _ => &self.own[..] };
        t[i]
    }
}
fn main() {
    let s = S { which: 0, own: [code(1, 1, 1); 2] };
    println!("{:?} {:?}", s.get(1), s.get(3));
}
