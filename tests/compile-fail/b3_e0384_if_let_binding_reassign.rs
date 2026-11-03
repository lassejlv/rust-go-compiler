//~ ERROR E0384
fn main() {
    let o = Some(3);
    if let Some(n) = o {
        n = n + 1;
        println!("{}", n);
    }
}
