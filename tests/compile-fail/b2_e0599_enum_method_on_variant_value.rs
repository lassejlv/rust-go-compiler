//~ ERROR E0599
enum Dir { Up, Down }
fn main() { let d = Dir::Up; d.flip(); }
