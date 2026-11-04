//~ ERROR E0367
struct Guard<T> { value: T }
impl<T: std::fmt::Debug> Drop for Guard<T> {
    fn drop(&mut self) { println!("{:?}", self.value); }
}
fn main() {}
