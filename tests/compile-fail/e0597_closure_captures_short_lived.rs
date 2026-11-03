//~ ERROR E0597
fn main() {
    let f;
    {
        let s = String::from("hi");
        f = || s.len();
    }
    println!("{}", f());
}
