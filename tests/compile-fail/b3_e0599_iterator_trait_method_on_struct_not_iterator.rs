//~ ERROR E0599
struct Count { n: u32 }
fn main() {
    let c = Count { n: 0 };
    let v: Vec<u32> = c.take(3).collect();
    println!("{:?} {}", v, 0);
}
