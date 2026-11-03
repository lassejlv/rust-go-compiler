//~ ERROR E0308
fn main() {
    let o = Some(2);
    let r = o.and_then(|x| x * 2);
    println!("{:?}", r);
}
