//~ ERROR E0053
trait T {
    fn f(&self, x: i32) -> i32;
}

struct S;

impl T for S {
    fn f(&self, x: u32) -> i32 {
        x as i32
    }
}

fn main() {
    println!("{}", S.f(1));
}
