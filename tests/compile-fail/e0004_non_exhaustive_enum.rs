//~ ERROR E0004
enum Dir {
    N,
    S,
    E,
    W,
}

fn name(d: Dir) -> &'static str {
    match d {
        Dir::N => "n",
        Dir::S => "s",
        Dir::E => "e",
    }
}

fn main() {
    println!("{}", name(Dir::W));
}
