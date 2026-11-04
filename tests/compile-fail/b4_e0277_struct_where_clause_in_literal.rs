//~ ERROR E0277
// A struct's where-clauses must hold for the types it is used with.
trait Unit { fn name() -> &'static str; }
struct Meters;
impl Unit for Meters { fn name() -> &'static str { "m" } }
struct Quantity<U: Unit> { value: f64, unit: U }
fn main() {
    let q = Quantity { value: 1.0, unit: 5u8 };
    println!("{}", q.value);
}
