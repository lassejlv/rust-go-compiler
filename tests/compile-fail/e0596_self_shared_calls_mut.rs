//~ ERROR E0596
struct C {
    v: Vec<i32>,
}

impl C {
    fn grow(&mut self) {
        self.v.push(0);
    }
    fn touch(&self) {
        self.grow();
    }
}

fn main() {
    let mut c = C { v: vec![] };
    c.touch();
    c.grow();
}
