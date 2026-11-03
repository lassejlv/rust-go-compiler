//~ ERROR E0605
fn main() {
    let s = String::from("1");
    let n = s as i32;
    println!("{}", n);
}
