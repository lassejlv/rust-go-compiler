//~ ERROR E0025
struct Pair { a: i32, b: i32 }
fn main() {
    let p = Pair { a: 1, b: 2 };
    let Pair { a, a: other, .. } = p;
    println!("{} {}", a, other);
}
