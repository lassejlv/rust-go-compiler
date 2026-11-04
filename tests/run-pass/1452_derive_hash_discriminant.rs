// derived Hash must feed the same bytes to the hasher as rustc
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

#[derive(Hash)] struct S { a: u8, b: u32, c: String }
#[derive(Hash)] struct T(i64, (), [u16; 2]);
#[derive(Hash)] enum E { A, B(u8), C { x: bool, y: char } }
#[derive(Hash)] enum One { Only(u32) }
#[derive(Hash)] #[repr(u8)] enum R { X = 3, Y }
#[derive(Hash)] struct G<T> { v: Vec<T>, o: Option<T> }
#[derive(Hash)] struct U;
#[derive(Hash)] struct Refs<'a> { s: &'a str, sl: &'a [i32] }

struct Recorder(Vec<String>);
impl Hasher for Recorder {
    fn finish(&self) -> u64 { 0 }
    fn write(&mut self, bytes: &[u8]) { self.0.push(format!("w{:?}", bytes)); }
    fn write_u8(&mut self, i: u8) { self.0.push(format!("u8:{}", i)); }
    fn write_u32(&mut self, i: u32) { self.0.push(format!("u32:{}", i)); }
    fn write_u64(&mut self, i: u64) { self.0.push(format!("u64:{}", i)); }
    fn write_usize(&mut self, i: usize) { self.0.push(format!("usize:{}", i)); }
    fn write_isize(&mut self, i: isize) { self.0.push(format!("isize:{}", i)); }
    fn write_i64(&mut self, i: i64) { self.0.push(format!("i64:{}", i)); }
}

fn rec<H: Hash>(h: &H) -> String { let mut r = Recorder(vec![]); h.hash(&mut r); r.0.join(" ") }
fn sip<H: Hash>(h: &H) -> u64 { let mut s = DefaultHasher::new(); h.hash(&mut s); s.finish() }

fn main() {
    println!("{}", rec(&S { a: 1, b: 2, c: "hi".into() }));
    println!("{}", rec(&T(-1, (), [7, 8])));
    println!("{}", rec(&E::A));
    println!("{}", rec(&E::B(9)));
    println!("{}", rec(&E::C { x: true, y: 'z' }));
    println!("{}", rec(&One::Only(4)));
    println!("{}", rec(&R::X));
    println!("{}", rec(&R::Y));
    println!("{}", rec(&G { v: vec![1u8, 2], o: Some(3u8) }));
    println!("{}", rec(&G::<u8> { v: vec![], o: None }));
    println!("{}", rec(&U));
    println!("{}", rec(&Refs { s: "ab", sl: &[5] }));
    println!("{}", rec(&(1u8, 2u32)));
    println!("{}", sip(&S { a: 1, b: 2, c: "hi".into() }));
    println!("{}", sip(&E::C { x: false, y: 'q' }));
    println!("{}", sip(&R::Y));
    println!("{}", sip(&"str"));
    println!("{}", sip(&(1i32, 'c', true)));
}
