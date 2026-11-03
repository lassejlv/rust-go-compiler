//~ ERROR E0384
struct P { x: i32, y: i32 }
fn main() {
    let P { x, y } = P { x: 1, y: 2 };
    x += y;
    println!("{}", x);
}
