//~ ERROR E0597
struct S {
    f: Box<dyn for<'r> Fn(&'r String) -> &'r str>,
}
impl S {
    fn run(&self) -> usize {
        let r;
        {
            let s = String::from("x");
            r = (self.f)(&s);
        }
        r.len()
    }
}
fn main() {
    println!("{}", S { f: Box::new(|s| s.as_str()) }.run());
}
