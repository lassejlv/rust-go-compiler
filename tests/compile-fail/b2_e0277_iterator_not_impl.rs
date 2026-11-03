//~ ERROR E0277
struct Counter { n: u32 }
fn main() {
    for x in (Counter { n: 0 }) { println!("{}", x); }
}
