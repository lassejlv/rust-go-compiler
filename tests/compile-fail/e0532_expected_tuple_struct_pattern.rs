//~ ERROR E0532
enum E {
    Unit,
    Tup(i32),
}

fn main() {
    let e = E::Tup(1);
    match e {
        E::Unit(x) => println!("{}", x),
        E::Tup(_) => {}
    }
}
