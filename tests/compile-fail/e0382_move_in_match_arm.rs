//~ ERROR E0382
fn main() {
    let s = Some(String::from("hi"));
    match s {
        Some(inner) => println!("{}", inner),
        None => {}
    }
    println!("{:?}", s);
}
