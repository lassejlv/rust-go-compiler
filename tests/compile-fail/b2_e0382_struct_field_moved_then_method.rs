//~ ERROR E0382
#[derive(Debug)]
struct Pair { a: String, b: String }
impl Pair { fn show(&self) { println!("{:?}", self); } }
fn main() {
    let p = Pair { a: "1".into(), b: "2".into() };
    let a = p.a;
    p.show();
    println!("{}", a);
}
