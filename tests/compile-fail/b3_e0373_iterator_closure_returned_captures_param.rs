//~ ERROR E0373
fn adder(n: i32) -> impl Iterator<Item = i32> {
    (0..3).map(|x| x + n)
}
fn main() { for x in adder(2) { println!("{}", x); } }
