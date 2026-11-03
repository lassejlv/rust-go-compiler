//~ ERROR E0502
#[derive(Debug, Clone, Copy)]
struct T<'a, 'b> { x: &'a str, y: &'b str, n: i32 }
fn main() {
    let mut a = String::from("a");
    let b = String::from("b");
    let t = T { y: &a, x: &b, n: 1 };
    let t2 = T { n: 2, ..t };
    let ty = t2.y;
    a.push('!');
    println!("{}", ty);
}
