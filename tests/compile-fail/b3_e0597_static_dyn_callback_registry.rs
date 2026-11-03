//~ ERROR E0597
fn register(_f: &'static dyn Fn()) {}
fn main() {
    let n = 5;
    let cb = move || println!("{}", n);
    register(&cb);
}
