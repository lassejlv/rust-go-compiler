#[derive(Debug)]
pub struct ChunkIter<'a, const MIN_ALIGN: usize = 1> { raw: &'a [u8] }
impl<'a, const MIN_ALIGN: usize> ChunkIter<'a, MIN_ALIGN> { fn n(&self) -> usize { MIN_ALIGN + self.raw.len() } }
fn main() { let c: ChunkIter = ChunkIter { raw: b"ab" }; println!("{:?} {}", c, c.n()); }
