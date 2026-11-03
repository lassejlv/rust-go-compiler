//~ ERROR E0063
struct Page<T> { items: Vec<T>, total: usize, next: Option<u32> }
fn main() { let p: Page<u8> = Page { items: vec![], total: 0 }; let _ = p.items; }
