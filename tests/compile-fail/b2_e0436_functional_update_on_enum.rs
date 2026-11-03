//~ ERROR E0436
enum E { V { a: i32, b: i32 } }
fn main() {
    let e = E::V { a: 1, b: 2 };
    let _ = E::V { a: 3, ..e };
}
