//~ ERROR E0515
fn label(n: u32) -> &'static str {
    format!("item{}", n).as_str()
}
fn main() { println!("{}", label(1)); }
