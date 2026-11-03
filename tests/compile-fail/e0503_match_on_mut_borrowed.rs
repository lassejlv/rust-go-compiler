//~ ERROR E0503
enum E {
    A(i32),
    B,
}

fn main() {
    let mut e = E::A(1);
    let r = &mut e;
    match e {
        E::A(_) => println!("a"),
        E::B => println!("b"),
    }
    *r = E::B;
}
