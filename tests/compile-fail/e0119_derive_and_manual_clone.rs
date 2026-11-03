//~ ERROR E0119
#[derive(Clone)]
struct S;

impl Clone for S {
    fn clone(&self) -> S {
        S
    }
}

fn main() {
    let _ = S.clone();
}
