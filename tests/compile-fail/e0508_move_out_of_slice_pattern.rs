//~ ERROR E0508
fn first(s: &[String]) -> String {
    s[0]
}

fn main() {
    println!("{}", first(&[String::from("a")]));
}
