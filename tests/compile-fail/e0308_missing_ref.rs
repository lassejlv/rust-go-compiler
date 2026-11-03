//~ ERROR E0308
fn len(s: &String) -> usize {
    s.len()
}

fn main() {
    let s = String::from("a");
    println!("{}", len(s));
}
