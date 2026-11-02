use std::arch::asm;
fn add(a: u64, b: u64) -> u64 {
    let r: u64;
    unsafe { asm!("lea {r}, [{a} + {b}]", a = in(reg) a, b = in(reg) b, r = out(reg) r); }
    r
}
fn getpid_raw() -> i64 {
    let ret: i64;
    unsafe { asm!("syscall", inlateout("rax") 39i64 => ret, lateout("rcx") _, lateout("r11") _, options(nostack)); }
    ret
}
fn main() {
    let mut x: u32 = 5;
    unsafe { asm!("add {0:e}, 3", inout(reg) x); }
    println!("{} {} {}", add(40, 2), x, getpid_raw() == std::process::id() as i64);
}
