//~ ERROR E0080
const fn div(a: i32, b: i32) -> i32 {
    a / b
}

const Q: i32 = div(1, 0);

fn main() {
    println!("{}", Q);
}
