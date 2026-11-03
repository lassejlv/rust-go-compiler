//~ ERROR E0308
fn f() -> String {
    42
}

fn main() {
    println!("{}", f());
}
