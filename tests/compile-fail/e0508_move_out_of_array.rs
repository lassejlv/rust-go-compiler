//~ ERROR E0508
fn main() {
    let a = [String::from("x"), String::from("y")];
    let i = 1;
    let s = a[i];
    println!("{}", s);
}
