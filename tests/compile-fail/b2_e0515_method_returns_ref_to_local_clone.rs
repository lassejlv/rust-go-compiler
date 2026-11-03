//~ ERROR E0515
struct S { v: Vec<i32> }
impl S {
    fn sorted(&self) -> &[i32] {
        let mut c = self.v.clone();
        c.sort();
        &c
    }
}
fn main() { println!("{:?}", S { v: vec![2, 1] }.sorted()); }
