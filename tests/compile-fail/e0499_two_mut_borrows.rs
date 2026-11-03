//~ ERROR E0499
fn main() {
    let mut s = String::from("a");
    let r1 = &mut s;
    let r2 = &mut s;
    r1.push('b');
    r2.push('c');
    println!("{}", s);
}
