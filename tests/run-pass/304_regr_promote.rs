struct VT { f: fn() -> usize, g: fn(i32) -> i32 }
fn sz<E>() -> usize { std::mem::size_of::<E>() }
fn dbl(x: i32) -> i32 { x * 2 }
fn make<E>() -> &'static VT { let vt = &VT { f: sz::<E>, g: dbl }; vt }
fn clobber(n: u64) -> u64 { let a = [n; 64]; a.iter().sum() }
fn main() {
    let a = make::<u64>();
    let b = make::<[u8; 3]>();
    println!("{}", clobber(3));
    println!("{} {} {}", (a.f)(), (b.f)(), (a.g)(21));
}
