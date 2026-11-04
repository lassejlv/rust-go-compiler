//~ ERROR E0366
struct Buffer<T> { items: Vec<T> }
impl Drop for Buffer<u8> {
    fn drop(&mut self) { println!("drop {}", self.items.len()); }
}
fn main() {}
