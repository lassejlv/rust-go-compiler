//~ ERROR E0015
// `for` loops are not allowed in const fns.
const fn sum(n: u32) -> u32 {
    let mut s = 0;
    for i in 0..n {
        s += i;
    }
    s
}
fn main() {
    println!("{}", sum(4));
}
