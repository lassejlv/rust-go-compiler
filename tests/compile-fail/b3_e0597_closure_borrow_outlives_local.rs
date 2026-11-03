//~ ERROR E0597
fn main() {
    let f;
    {
        let prefix = String::from("> ");
        f = |s: &str| format!("{}{}", prefix, s);
    }
    println!("{}", f("x"));
}
