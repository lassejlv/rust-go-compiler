//~ ERROR E0596
fn main() {
    let mut n = 0;
    let inc = || n += 1;
    inc();
    println!("{}", n);
}
