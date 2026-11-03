//~ ERROR E0277
use std::fmt::Display;

fn show<T: Display>(t: T) {
    println!("{}", t);
}

struct Opaque;

fn main() {
    show(Opaque);
}
