//~ ERROR E0609
struct P { x: i32 }
fn main() {
    let v = vec![P { x: 1 }];
    println!("{}", v.first().x);
}
