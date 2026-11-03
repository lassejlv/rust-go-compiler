//~ ERROR E0502
struct S {
    v: Vec<i32>,
}

impl S {
    fn total(&self) -> i32 {
        self.v.iter().sum()
    }
}

fn main() {
    let mut s = S { v: vec![1, 2] };
    let r = &mut s.v;
    let t = s.total();
    r.push(t);
}
