#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq)]
enum Opcode {
    Nop = 0,
    Push = 0x10,
    Pop,
    Add = 0x20,
    Halt = 0xFF,
}

impl Opcode {
    fn from_u8(b: u8) -> Option<Opcode> {
        match b {
            0 => Some(Opcode::Nop),
            0x10 => Some(Opcode::Push),
            0x11 => Some(Opcode::Pop),
            0x20 => Some(Opcode::Add),
            0xFF => Some(Opcode::Halt),
            _ => None,
        }
    }
}

impl TryFrom<u8> for Opcode {
    type Error = String;
    fn try_from(b: u8) -> Result<Self, String> {
        Opcode::from_u8(b).ok_or_else(|| format!("bad opcode {:#04x}", b))
    }
}

#[repr(i16)]
#[derive(Debug)]
enum Temp {
    Cold = -40,
    Mild = 20,
    Hot = 45,
}

#[derive(Debug, Clone, Copy)]
enum Implicit {
    A,
    B,
    C = 10,
    D,
}

#[repr(C)]
union IntOrFloat {
    i: u32,
    f: f32,
}

#[repr(C)]
union Bytes4 {
    word: u32,
    bytes: [u8; 4],
    halves: [u16; 2],
}

#[repr(C)]
#[derive(Clone, Copy)]
struct Tagged {
    is_float: bool,
    val: Val,
}

#[repr(C)]
#[derive(Clone, Copy)]
union Val {
    i: i64,
    f: f64,
}

fn show(t: &Tagged) -> String {
    unsafe {
        if t.is_float {
            format!("float {}", t.val.f)
        } else {
            format!("int {}", t.val.i)
        }
    }
}

fn main() {
    let ops = [Opcode::Nop, Opcode::Push, Opcode::Pop, Opcode::Add, Opcode::Halt];
    for op in ops {
        println!("{:?} = {} ({:#x})", op, op as u8, op as u8);
    }
    for b in [0x10u8, 0x11, 0x12, 0xFF] {
        println!("{:#04x} -> {:?}", b, Opcode::try_from(b));
    }
    println!("size_of Opcode {}", std::mem::size_of::<Opcode>());
    println!("{} {} {}", Temp::Cold as i16, Temp::Mild as i32, Temp::Hot as u8);
    println!("{:?} {}", Temp::Cold, std::mem::size_of::<Temp>());
    println!("{} {} {} {}", Implicit::A as i32, Implicit::B as i32, Implicit::C as i32, Implicit::D as i32);
    println!("{}", Opcode::Halt as u8 as i8);

    let u = IntOrFloat { f: 1.0 };
    println!("1.0 bits = {:#x}", unsafe { u.i });
    let u = IntOrFloat { i: 0xC0490FDB };
    println!("float = {}", unsafe { u.f });
    let mut b = Bytes4 { word: 0 };
    b.bytes = [0x78, 0x56, 0x34, 0x12];
    unsafe {
        println!("word {:#x}", b.word);
        println!("halves {:#x} {:#x}", b.halves[0], b.halves[1]);
        b.halves[1] = 0xABCD;
        println!("word {:#x}", b.word);
        println!("bytes {:?}", b.bytes);
    }
    println!("union size {}", std::mem::size_of::<Bytes4>());

    let vals = [
        Tagged { is_float: false, val: Val { i: -12 } },
        Tagged { is_float: true, val: Val { f: 2.5 } },
        Tagged { is_float: false, val: Val { i: 1 << 40 } },
    ];
    for v in &vals {
        println!("{}", show(v));
    }
    let mut t = vals[1];
    unsafe {
        t.val.f *= 4.0;
        println!("modified {}", t.val.f);
        match t {
            Tagged { is_float: true, val: Val { f } } if f > 5.0 => println!("big float {}", f),
            _ => println!("other"),
        }
    }
}
