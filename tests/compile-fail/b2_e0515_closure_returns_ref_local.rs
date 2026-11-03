//~ ERROR E0515
fn main() {
    let f = || -> &'static str {
        let s = String::from("x");
        &s
    };
    println!("{}", f());
}
