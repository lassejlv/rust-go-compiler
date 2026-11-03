//~ ERROR E0080
const ARR: [i32; 3] = [1, 2, 3];
const X: i32 = ARR[5];

fn main() {
    println!("{}", X);
}
