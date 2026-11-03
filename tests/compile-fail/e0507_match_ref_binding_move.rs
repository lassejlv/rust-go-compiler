//~ ERROR E0507
fn get(o: &Option<String>) -> String {
    match *o {
        Some(s) => s,
        None => String::new(),
    }
}

fn main() {
    println!("{}", get(&Some("q".into())));
}
