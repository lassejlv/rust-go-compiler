//~ ERROR E0081
// C gets the implicit discriminant 1, which A already has.
enum Code {
    A = 1,
    B = 0,
    C,
}
fn main() { println!("{}", Code::A as i32); }
