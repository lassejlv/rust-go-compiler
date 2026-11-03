fn rec(n: u64) -> u64 { let a = [n; 64]; if n == 0 { 0 } else { rec(n - 1) + std::hint::black_box(a)[3] } }
fn main() {
    println!("before overflow");
    println!("{}", rec(std::hint::black_box(100_000_000)));
}
