//~ ERROR E0594
fn run<F: Fn()>(f: F) { f(); }
fn main() {
    let mut x = 0;
    run(|| x = 1);
    println!("{}", x);
}
