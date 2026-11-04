//~ ERROR E0277
// The same for an enum variant's field.
#[derive(PartialEq, Eq, Debug)]
enum Value {
    Int(i64),
    Float(f64),
}
fn main() {
    println!("{:?}", Value::Int(1) == Value::Float(1.0));
}
