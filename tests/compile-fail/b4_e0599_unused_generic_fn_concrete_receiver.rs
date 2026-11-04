//~ ERROR E0599
fn describe<T: std::fmt::Debug>(x: T) -> String {
    let s = format!("{:?}", x);
    s.to_upper()
}
fn main() {}
