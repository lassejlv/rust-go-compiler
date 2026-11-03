//~ ERROR E0061
struct C;

impl C {
    fn go(&self, n: i32) -> i32 {
        n
    }
}

fn main() {
    println!("{}", C.go(1, 2));
}
