//~ ERROR E0503
fn main() {
    let mut count = 0;
    let mut inc = || count += 1;
    count += 10;
    inc();
}
