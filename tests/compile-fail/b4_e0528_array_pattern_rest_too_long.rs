//~ ERROR E0528
fn main() {
    let pair = [1, 2];
    let [a, b, c, ..] = pair;
    println!("{} {} {}", a, b, c);
}
