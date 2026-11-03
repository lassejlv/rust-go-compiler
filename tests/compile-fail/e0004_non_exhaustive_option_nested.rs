//~ ERROR E0004
fn main() {
    let o: Option<Option<i32>> = Some(None);
    match o {
        Some(Some(x)) => println!("{}", x),
        None => println!("none"),
    }
}
