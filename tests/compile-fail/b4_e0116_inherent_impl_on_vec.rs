//~ ERROR E0116
impl Vec<u8> {
    fn checksum(&self) -> u32 { self.iter().map(|&b| b as u32).sum() }
}
fn main() {}
