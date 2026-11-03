//~ ERROR E0631
fn apply<F: Fn(&str) -> usize>(f: F) -> usize {
    f("abc")
}

fn main() {
    println!("{}", apply(|s: String| s.len()));
}
