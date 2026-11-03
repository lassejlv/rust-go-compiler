//~ ERROR E0499
fn main() {
    let mut count = 0;
    let mut inc = || count += 1;
    let r = &mut count;
    inc();
    *r += 1;
}
