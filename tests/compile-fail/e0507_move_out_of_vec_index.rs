//~ ERROR E0507
fn main() {
    let v = vec![String::from("a"), String::from("b")];
    let s = v[0];
    println!("{}", s);
}
