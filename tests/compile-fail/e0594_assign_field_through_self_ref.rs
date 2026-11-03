//~ ERROR E0594
struct Counter {
    n: u32,
}

impl Counter {
    fn reset(&self) {
        self.n = 0;
    }
}

fn main() {
    let c = Counter { n: 3 };
    c.reset();
}
