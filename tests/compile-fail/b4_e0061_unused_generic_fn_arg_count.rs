//~ ERROR E0061
fn scale(x: f64, by: f64) -> f64 { x * by }
fn scale_all<T: Into<f64>>(v: Vec<T>) -> Vec<f64> {
    v.into_iter().map(|x| scale(x.into())).collect()
}
fn main() {}
