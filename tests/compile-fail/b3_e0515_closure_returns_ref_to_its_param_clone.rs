//~ ERROR E0515
fn main() {
    let f = |s: &String| -> &str { let c = s.clone(); &c };
    let x = String::from("a");
    println!("{}", f(&x));
}
