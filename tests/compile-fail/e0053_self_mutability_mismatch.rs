//~ ERROR E0053
trait Reset {
    fn reset(&mut self);
}

struct C(i32);

impl Reset for C {
    fn reset(&self) {}
}

fn main() {
    let mut c = C(1);
    c.reset();
    println!("{}", c.0);
}
