//~ ERROR E0716
fn main() {
    let parts: Vec<&str>;
    parts = ["a", "b"].join(",").split(',').collect();
    println!("{:?}", parts);
}
