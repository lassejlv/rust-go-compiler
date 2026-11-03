//~ ERROR E0597
fn main() {
    let f: Box<dyn Fn() -> usize>;
    {
        let s = String::from("abc");
        let r = &s;
        f = Box::new(move || r.len());
    }
    println!("{}", f());
}
