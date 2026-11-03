//~ ERROR E0624
mod store {
    pub struct Store<T> { v: Vec<T> }
    impl<T> Store<T> {
        pub fn new() -> Self { Store { v: Vec::new() } }
        fn raw(&self) -> &Vec<T> { &self.v }
    }
}
fn main() { let s: store::Store<i32> = store::Store::new(); s.raw(); }
