//~ ERROR overflowing_literals
const BIG: f32 = 1e39;
fn main() {
    println!("{}", BIG);
}
