//~ ERROR E0794
// Explicit lifetime arguments for a function with late-bound lifetimes.
fn first<'a>(x: &'a str, _y: &str) -> &'a str {
    x
}
fn main() {
    println!("{}", first::<'static>("a", "b"));
}
