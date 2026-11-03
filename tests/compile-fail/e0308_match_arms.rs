//~ ERROR E0308
fn main() {
    let n = 3;
    let s = match n {
        0 => "zero",
        _ => n,
    };
    println!("{}", s);
}
