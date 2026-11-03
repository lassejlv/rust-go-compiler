//~ ERROR E0505
struct Parser<'a> { input: &'a str }
fn main() {
    let text = String::from("1 2 3");
    let p = Parser { input: &text };
    drop(text);
    println!("{}", p.input);
}
