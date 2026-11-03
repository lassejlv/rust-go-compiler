// arithmetic overflow panics in debug builds (exit 101) after some checked/wrapping work
fn factorial_checked(n: u32) -> Option<u32> {
    (1..=n).try_fold(1u32, |acc, x| acc.checked_mul(x))
}

fn sum_squares(v: &[u8]) -> u8 {
    let mut s: u8 = 0;
    for &x in v {
        s += x * x / 4;
        println!("running sum {}", s);
    }
    s
}

fn main() {
    for n in [5, 10, 12, 13, 20] {
        println!("{}! = {:?}", n, factorial_checked(n));
    }
    println!("{}", 250u8.wrapping_add(10));
    println!("{:?}", 250u8.overflowing_add(10));
    println!("{}", i32::MAX.saturating_add(5));
    println!("{:?}", (-128i8).checked_neg());
    println!("{:?}", (-128i8).checked_abs());
    println!("{}", (-128i8).wrapping_abs());
    println!("{:?}", 5u32.checked_sub(6));
    let r = std::panic::catch_unwind(|| {
        let x: i32 = std::hint::black_box(i32::MIN);
        x / -1
    });
    println!("div overflow caught: {}", r.is_err());
    let r2 = std::panic::catch_unwind(|| {
        let x: u64 = std::hint::black_box(0);
        10 % x
    });
    println!("rem by zero caught: {}", r2.is_err());
    let r3 = std::panic::catch_unwind(|| {
        let s: u32 = std::hint::black_box(40);
        1u32 << s
    });
    println!("shl overflow caught: {}", r3.is_err());
    let r4 = std::panic::catch_unwind(|| {
        let a: i16 = std::hint::black_box(-32768);
        -a
    });
    println!("neg overflow caught: {}", r4.is_err());
    let r5 = std::panic::catch_unwind(|| {
        let a: usize = std::hint::black_box(3);
        a - 4
    });
    println!("usize underflow caught: {}", r5.is_err());
    let r6 = std::panic::catch_unwind(|| {
        let a: i64 = std::hint::black_box(i64::MAX);
        a.pow(2)
    });
    println!("pow overflow caught: {}", r6.is_err());
    let r7 = std::panic::catch_unwind(|| {
        let a: u8 = std::hint::black_box(200);
        a as i8
    });
    println!("as cast does not panic: {:?}", r7);
    let r8 = std::panic::catch_unwind(|| {
        let v: Vec<u16> = std::hint::black_box(vec![60000, 6000]);
        v.iter().sum::<u16>()
    });
    println!("sum overflow caught: {}", r8.is_err());
    let r9 = std::panic::catch_unwind(|| {
        let a: i8 = std::hint::black_box(-100);
        a - 100
    });
    println!("i8 sub overflow caught: {}", r9.is_err());
    let r10 = std::panic::catch_unwind(|| {
        let a: u32 = std::hint::black_box(1);
        let s: u32 = std::hint::black_box(32);
        a >> s
    });
    println!("shr overflow caught: {}", r10.is_err());
    let r11 = std::panic::catch_unwind(|| {
        let a: i32 = std::hint::black_box(i32::MIN);
        a.abs()
    });
    println!("abs overflow caught: {}", r11.is_err());
    let r12 = std::panic::catch_unwind(|| {
        let v: Vec<u8> = std::hint::black_box(vec![16, 16]);
        v.iter().product::<u8>()
    });
    println!("product overflow caught: {}", r12.is_err());
    let r13 = std::panic::catch_unwind(|| {
        let mut x: u16 = std::hint::black_box(65535);
        x += 1;
        x
    });
    println!("add-assign overflow caught: {}", r13.is_err());
    println!("final: {}", sum_squares(&std::hint::black_box([10u8, 20, 30, 40])));
    println!("unreachable");
}
