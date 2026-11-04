//~ ERROR E0384
trait Counter {
    fn get(&self) -> u32;
    fn bumped(&self) -> u32 {
        let n = self.get();
        n = n + 1;
        n
    }
}
fn main() {}
