//~ ERROR E0716
struct View<'a> { text: &'a str }
fn main() {
    let v;
    v = View { text: &String::from("tmp") };
    println!("{}", v.text);
}
