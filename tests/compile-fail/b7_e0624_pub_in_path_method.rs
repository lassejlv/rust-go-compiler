//~ ERROR E0624
// A method restricted with `pub(in path)` called outside that module.
mod a {
    pub struct S;
    impl S {
        pub(in crate::a) fn m(&self) {}
    }
}
fn main() {
    a::S.m();
}
