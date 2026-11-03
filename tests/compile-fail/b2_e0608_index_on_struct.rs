//~ ERROR E0608
struct Grid { cells: Vec<u8> }
fn main() { let g = Grid { cells: vec![0] }; println!("{}", g[0]); }
