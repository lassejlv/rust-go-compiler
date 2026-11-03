//~ ERROR E0515
fn make() -> &'static String {
    let s = String::from("local");
    &s
}

fn main() {
    println!("{}", make());
}
