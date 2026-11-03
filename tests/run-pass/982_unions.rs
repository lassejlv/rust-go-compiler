#[repr(C)]
union IntOrFloat {
    i: u32,
    f: f32,
}

#[repr(C)]
#[derive(Clone, Copy)]
union Bytes8 {
    v: u64,
    b: [u8; 8],
    h: [u16; 4],
}

#[derive(Clone, Copy)]
enum Tag { I, F }

struct Tagged {
    tag: Tag,
    val: IntOrFloat,
}

impl Tagged {
    fn show(&self) -> String {
        unsafe {
            match self.tag {
                Tag::I => format!("I({})", self.val.i),
                Tag::F => format!("F({})", self.val.f),
            }
        }
    }
}

union WithDrop {
    a: std::mem::ManuallyDrop<String>,
    n: u8,
}

fn main() {
    let mut u = IntOrFloat { f: 1.0 };
    unsafe {
        println!("bits of 1.0 = {:#x}", u.i);
        u.i = 0x40490fdb;
        println!("float = {}", u.f);
    }
    let b = Bytes8 { v: 0x0102030405060708 };
    unsafe {
        println!("{:?}", b.b);
        println!("{:?}", b.h);
    }
    let c = b;
    unsafe { println!("copy v={:#018x}", c.v); }
    println!("size {} align {}", std::mem::size_of::<Bytes8>(), std::mem::align_of::<Bytes8>());
    let items = vec![
        Tagged { tag: Tag::I, val: IntOrFloat { i: 42 } },
        Tagged { tag: Tag::F, val: IntOrFloat { f: 2.5 } },
    ];
    for t in &items {
        println!("{}", t.show());
    }
    let mut w = WithDrop { a: std::mem::ManuallyDrop::new(String::from("hello")) };
    unsafe {
        println!("w.a = {}", *w.a);
        std::mem::ManuallyDrop::drop(&mut w.a);
        w.n = 5;
        println!("w.n = {}", w.n);
    }
    let u2 = IntOrFloat { i: 3 };
    let unsafe_i = unsafe { let IntOrFloat { i } = IntOrFloat { i: 77 }; i };
    println!("destructured {}", unsafe_i);
    unsafe {
        match u2 {
            IntOrFloat { i: 3 } => println!("matched 3"),
            IntOrFloat { i } => println!("other {}", i),
        }
    }
}
