//~ ERROR E0408
enum E {
    A(i32),
    B(i32),
    C,
}

fn main() {
    let e = E::C;
    match e {
        E::A(x) | E::C => println!("{}", x),
        E::B(_) => {}
    }
}
