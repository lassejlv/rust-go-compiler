//~ ERROR E0505
enum Msg { Text(String), Quit }
fn main() {
    let m = Msg::Text("hi".into());
    let r = &m;
    let owned = match m { Msg::Text(s) => s, Msg::Quit => String::new() };
    if let Msg::Text(t) = r { println!("{} {}", t, owned); }
}
