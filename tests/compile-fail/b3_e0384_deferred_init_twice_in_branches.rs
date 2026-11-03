//~ ERROR E0384
fn main() {
    let msg: &str;
    msg = "a";
    if true { msg = "b"; }
    println!("{}", msg);
}
