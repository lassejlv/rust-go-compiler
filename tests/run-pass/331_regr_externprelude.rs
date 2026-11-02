extern crate alloc as core_alloc;
mod m {
    use core_alloc::borrow::Cow;
    pub fn f() -> Cow<'static, str> { Cow::Borrowed("hi") }
}
fn main() { println!("{}", m::f()); }
