//~ ERROR E0502
fn first_word(s: &str) -> &str {
    s.split(' ').next().unwrap_or("")
}

fn main() {
    let mut s = String::from("hello world");
    let w = first_word(&s);
    s.clear();
    println!("{}", w);
}
