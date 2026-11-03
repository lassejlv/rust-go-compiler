//~ ERROR E0277
fn main() {
    let s = String::from("1");
    if s == 1 { println!("eq"); }
}
