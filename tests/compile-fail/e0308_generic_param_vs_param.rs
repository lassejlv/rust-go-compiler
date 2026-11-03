//~ ERROR E0308
// Two type parameters are different types, even when every caller passes
// the same type for both.
fn first<A, B>(a: A, _b: B) -> B {
    a
}

fn main() {
    println!("{}", first(1, 2));
}
