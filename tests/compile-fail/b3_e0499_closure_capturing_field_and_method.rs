//~ ERROR E0499
struct S { v: Vec<i32>, n: i32 }
impl S { fn inc(&mut self) { self.n += 1; } }
fn main() {
    let mut s = S { v: vec![], n: 0 };
    let mut push = || s.v.push(1);
    s.inc();
    push();
}
