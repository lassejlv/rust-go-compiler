//~ ERROR E0308
fn main() {
    let s = "abc";
    let n = s.bytes().filter(|b| *b == 'a').count();
    println!("{}", n);
}
