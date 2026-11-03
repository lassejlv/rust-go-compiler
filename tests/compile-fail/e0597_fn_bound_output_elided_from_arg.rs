//~ ERROR E0597
fn apply<F: Fn(&String) -> &str>(f: F) -> usize {
    let r;
    {
        let s = String::from("abc");
        r = f(&s);
    }
    r.len()
}
fn main() {
    println!("{}", apply(|s| s.as_str()));
}
