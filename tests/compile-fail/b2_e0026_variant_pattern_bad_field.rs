//~ ERROR E0026
enum E { S { a: i32 } }
fn main() {
    let e = E::S { a: 1 };
    match e { E::S { b } => println!("{}", b) }
}
