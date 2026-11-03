//~ ERROR E0424
struct S {
    v: i32,
}

impl S {
    fn get() -> i32 {
        self.v
    }
}

fn main() {
    println!("{}", S::get());
}
