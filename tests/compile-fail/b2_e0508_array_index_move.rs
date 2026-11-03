//~ ERROR E0508
fn main() {
    let a = [String::from("a"), String::from("b")];
    let s = a[0];
    println!("{}", s);
}
