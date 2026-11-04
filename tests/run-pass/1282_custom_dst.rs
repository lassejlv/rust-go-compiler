// Custom DSTs: struct with trailing [T] / str / dyn Trait, unsizing coercions, raw slice pointers.
use std::fmt::Debug;
use std::mem::size_of_val;

struct Packet<T: ?Sized> {
    id: u16,
    payload: T,
}

#[derive(Debug)]
struct Labeled<T: ?Sized + Debug> {
    tag: char,
    val: T,
}

trait Describe {
    fn describe(&self) -> String;
}
impl Describe for i32 {
    fn describe(&self) -> String {
        format!("int {}", self)
    }
}
impl Describe for String {
    fn describe(&self) -> String {
        format!("string {:?}", self)
    }
}

fn total(p: &Packet<[u32]>) -> u32 {
    p.payload.iter().sum()
}

#[repr(transparent)]
struct Path2(str);
impl Path2 {
    fn new(s: &str) -> &Path2 {
        unsafe { &*(s as *const str as *const Path2) }
    }
    fn ext(&self) -> Option<&str> {
        self.0.rsplit_once('.').map(|(_, e)| e)
    }
}

#[repr(transparent)]
struct Bits([u8]);
impl Bits {
    fn from_mut(b: &mut [u8]) -> &mut Bits {
        unsafe { &mut *(b as *mut [u8] as *mut Bits) }
    }
    fn set(&mut self, i: usize) {
        self.0[i / 8] |= 1 << (i % 8);
    }
    fn count(&self) -> u32 {
        self.0.iter().map(|b| b.count_ones()).sum()
    }
}

fn main() {
    let p: Packet<[u32; 4]> = Packet { id: 7, payload: [1, 2, 3, 4] };
    let dp: &Packet<[u32]> = &p;
    println!("id {} len {} total {}", dp.id, dp.payload.len(), total(dp));
    println!("size_of_val {} {}", size_of_val(&p), size_of_val(dp));
    let boxed: Box<Packet<[u32]>> = Box::new(Packet { id: 9, payload: [10, 20] });
    println!("boxed total {}", total(&boxed));

    let l: Box<Labeled<dyn Debug>> = Box::new(Labeled { tag: 'x', val: vec![1, 2] });
    println!("{:?}", l);
    let items: Vec<Box<Packet<dyn Describe>>> = vec![
        Box::new(Packet { id: 1, payload: 5i32 }),
        Box::new(Packet { id: 2, payload: String::from("hi") }),
    ];
    for it in &items {
        println!("{} -> {}", it.id, it.payload.describe());
    }
    let r: &Labeled<[char]> = &Labeled { tag: 'z', val: ['a', 'b', 'c'] };
    println!("{:?} {}", r, r.val.len());

    let path = Path2::new("archive.tar.gz");
    println!("{:?} {:?}", path.ext(), Path2::new("noext").ext());
    let mut storage = [0u8; 4];
    let bits = Bits::from_mut(&mut storage);
    for i in [0, 3, 9, 31, 3] {
        bits.set(i);
    }
    println!("bits {} {:?}", bits.count(), storage);

    let arr = [1.5f64, 2.5, 3.5];
    let raw: *const [f64] = &arr[1..];
    let raw2 = std::ptr::slice_from_raw_parts(arr.as_ptr(), 2);
    unsafe {
        println!("{:?} {:?}", &*raw, &*raw2);
    }
    let s: &str = "héllo";
    let bytes: *const [u8] = s.as_bytes();
    println!("{} {}", bytes.len(), s.chars().count());
    let rc: std::rc::Rc<[i32]> = std::rc::Rc::from(vec![3, 1, 2]);
    let rcd: std::rc::Rc<dyn Debug> = std::rc::Rc::new(5u8);
    println!("{:?} {:?}", rc, rcd);
}
