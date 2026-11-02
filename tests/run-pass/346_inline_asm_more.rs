use std::arch::asm;
fn popcnt_loop(mut x: u64) -> u64 {
    let mut n: u64 = 0;
    unsafe {
        asm!(
            "2:",
            "test {x}, {x}",
            "jz 3f",
            "mov {t}, {x}",
            "sub {t}, 1",
            "and {x}, {t}",
            "inc {n}",
            "jmp 2b",
            "3:",
            x = inout(reg) x,
            n = inout(reg) n,
            t = out(reg) _,
        );
    }
    let _ = x;
    n
}
fn shl_const(x: u32) -> u32 {
    let r: u32;
    unsafe { asm!("mov {0:e}, {1:e}", "shl {0:e}, {k}", out(reg) r, in(reg) x, k = const 4); }
    r
}
fn add_f64(a: f64, b: f64) -> f64 {
    let r: f64;
    unsafe { asm!("addsd {0}, {1}", inout(xmm_reg) a => r, in(xmm_reg) b); }
    r
}
fn r8_test(v: u64) -> u64 {
    let out: u64;
    unsafe { asm!("lea {o}, [r8 + r9*2]", o = out(reg) out, in("r8") v, in("r9") 10u64); }
    out
}
fn att(x: u64) -> u64 {
    let r: u64;
    unsafe { asm!("movq {1}, {0}", "addq $7, {0}", out(reg) r, in(reg) x, options(att_syntax)); }
    r
}
fn main() {
    println!("{} {} {} {} {}", popcnt_loop(0b1011_0110), shl_const(3), add_f64(1.25, 2.5), r8_test(5), att(35));
}
