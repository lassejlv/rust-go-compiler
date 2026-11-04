// Deny-by-default lints switched off with #[allow]: the programs compile
// and behave as at run time (literals wrap, arithmetic panics).
#![allow(overflowing_literals)]

use std::panic;

fn wrapped_literals() {
    let a: u8 = 256;
    let b: i8 = 0xFF;
    let c: i8 = 200;
    let d: u16 = 70000;
    let e = 0xFFFF_FFFFi32;
    let f: i64 = 0x8000_0000_0000_0000;
    let g = 1e39f32;
    println!("{} {} {} {} {} {} {}", a, b, c, d, e, f, g);
    match 300u16 as u8 {
        44 => println!("cast wraps to 44"),
        _ => println!("other"),
    }
}

#[allow(arithmetic_overflow)]
fn overflowing_arith() -> Vec<bool> {
    let r1 = panic::catch_unwind(|| {
        let x: u8 = 255 + 1;
        x
    });
    let r2 = panic::catch_unwind(|| {
        let y = 1i32 << 40;
        y
    });
    vec![r1.is_err(), r2.is_err()]
}

#[allow(unconditional_panic)]
fn unconditional() -> Vec<bool> {
    let r1 = panic::catch_unwind(|| {
        let a = [1, 2, 3];
        a[3]
    });
    let r2 = panic::catch_unwind(|| {
        let n = std::hint::black_box(10u32);
        n / 0
    });
    let r3 = panic::catch_unwind(|| {
        let n = std::hint::black_box(10i64);
        n % 0
    });
    let r4 = panic::catch_unwind(|| -2147483648i32 / -1);
    vec![r1.is_err(), r2.is_err(), r3.is_err(), r4.is_err()]
}

mod inner {
    #![allow(arithmetic_overflow, unconditional_panic)]
    pub fn both() -> bool {
        std::panic::catch_unwind(|| {
            let v = [0u8; 2];
            v[2] + (250u8 + 10)
        })
        .is_err()
    }
}

fn main() {
    panic::set_hook(Box::new(|_| {}));
    wrapped_literals();
    println!("{:?}", overflowing_arith());
    println!("{:?}", unconditional());
    println!("{}", inner::both());
    // in range: no lint
    let ok: [i8; 3] = [-128, 127, -0];
    let ok2 = (255u8, 65535u16, -2147483648i32, 18446744073709551615u64);
    let ok3 = i128::MIN + 1;
    let ok4 = [10u8; 4][3] / 2 + (200 - 100);
    println!("{:?} {:?} {} {}", ok, ok2, ok3, ok4);
}
