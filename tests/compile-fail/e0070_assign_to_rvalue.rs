//~ ERROR E0070
fn one() -> i32 {
    1
}

fn main() {
    one() = 2;
}
