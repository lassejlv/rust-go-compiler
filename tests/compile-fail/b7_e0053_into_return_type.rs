//~ ERROR E0053
// `Into<i32>::into` returns `i32`.
struct Meters(i32);
impl Into<i32> for Meters {
    fn into(self) -> i64 {
        self.0 as i64
    }
}
fn main() {}
