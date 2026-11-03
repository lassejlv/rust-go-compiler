//~ ERROR E0382
enum Msg { Text(String), Quit }
fn main() {
    let m = Msg::Text(String::from("hi"));
    match m {
        Msg::Text(s) => println!("{}", s),
        Msg::Quit => {}
    }
    if let Msg::Text(t) = m { println!("{}", t); }
}
