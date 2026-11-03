//~ ERROR E0046
struct Celsius(f64);
impl From<f64> for Celsius {}
fn main() {}
