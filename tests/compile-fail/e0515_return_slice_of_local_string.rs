//~ ERROR E0515
fn greeting<'a>(name: &'a str) -> &'a str {
    let s = format!("hello {}", name);
    s.as_str()
}

fn main() {
    println!("{}", greeting("bob"));
}
