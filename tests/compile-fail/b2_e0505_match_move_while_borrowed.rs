//~ ERROR E0505
fn main() {
    let o = Some(String::from("x"));
    let r = o.as_ref();
    match o {
        Some(s) => println!("{}", s),
        None => {}
    }
    println!("{:?}", r);
}
