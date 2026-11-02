mod private {
    pub trait IdentDe { type D; fn from(self) -> Self::D; }
    impl IdentDe for u64 { type D = String; fn from(self) -> String { self.to_string() } }
}
fn main() {
    let a: u64 = u64::from(b'a');
    println!("{} {}", a, private::IdentDe::from(5u64));
}
