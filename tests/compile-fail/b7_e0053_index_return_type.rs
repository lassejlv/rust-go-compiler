//~ ERROR E0053
// `index` returns `&Self::Output`.
use std::ops::Index;
struct Table(Vec<i32>);
impl Index<usize> for Table {
    type Output = i32;
    fn index(&self, i: usize) -> &u8 {
        todo!()
    }
}
fn main() {}
