//~ ERROR E0603
mod m {
    pub mod inner { pub fn f() {} }
    use self::inner::f;
}
fn main() { m::f(); }
