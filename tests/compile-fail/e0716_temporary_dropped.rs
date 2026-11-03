//~ ERROR E0716
fn main() {
    let r: &String;
    r = &String::from("temp");
    println!("{}", r);
}
