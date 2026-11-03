//~ ERROR lifetime
struct Win<'a> { data: &'a mut [i32], pos: usize }
impl<'a> Iterator for Win<'a> {
    type Item = &'a mut i32;
    fn next(&mut self) -> Option<Self::Item> {
        let p = self.pos; self.pos += 1;
        self.data.get_mut(p)
    }
}
fn main() { let mut a = [1]; let w = Win { data: &mut a, pos: 0 }; for _ in w {} }
