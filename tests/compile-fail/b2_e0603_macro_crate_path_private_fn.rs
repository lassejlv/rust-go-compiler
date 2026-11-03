//~ ERROR E0603
mod util {
    fn hidden() -> i32 { 5 }
    macro_rules! h { () => { $crate::util::hidden() }; }
    pub(crate) use h;
}
fn main() { println!("{}", util::h!()); }
