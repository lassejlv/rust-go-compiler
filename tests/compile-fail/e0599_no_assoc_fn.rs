//~ ERROR E0599
struct S;

impl S {
    fn make() -> S {
        S
    }
}

fn main() {
    let _s = S::create();
}
