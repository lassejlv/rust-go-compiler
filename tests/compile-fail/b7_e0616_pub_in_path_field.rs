//~ ERROR E0616
// A field restricted with `pub(in path)` read outside that module.
mod a {
    pub mod b {
        pub struct S {
            pub(in crate::a) x: i32,
        }
        pub fn mk() -> S {
            S { x: 1 }
        }
    }
}
fn main() {
    let s = a::b::mk();
    println!("{}", s.x);
}
