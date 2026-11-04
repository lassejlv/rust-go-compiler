//~ ERROR E0603
// `pub(in crate::a)` is not visible outside `a`.
mod a {
    pub mod b {
        pub(in crate::a) fn f() {}
    }
    pub fn g() {
        b::f();
    }
}
fn main() {
    a::g();
    a::b::f();
}
