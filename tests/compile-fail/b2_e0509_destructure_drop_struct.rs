//~ ERROR E0509
struct Guard { name: String }
impl Drop for Guard { fn drop(&mut self) { println!("drop"); } }
fn main() {
    let g = Guard { name: "x".into() };
    let Guard { name } = g;
    println!("{}", name);
}
