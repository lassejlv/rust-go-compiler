//~ ERROR E0499
struct Counter {
    n: i32,
}

impl Counter {
    fn bump(&mut self, by: &mut i32) {
        self.n += *by;
    }
}

fn main() {
    let mut c = Counter { n: 0 };
    c.bump(&mut c.n);
}
