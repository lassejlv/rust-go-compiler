//~ ERROR E0308
fn same<T>(a: T, b: T) -> T {
    if true { a } else { b }
}

fn main() {
    println!("{}", same(1, "x"));
}
