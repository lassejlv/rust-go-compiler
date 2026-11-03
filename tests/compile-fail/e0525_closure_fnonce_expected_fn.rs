//~ ERROR E0525
fn run<F: Fn()>(f: F) {
    f();
}

fn main() {
    let s = String::from("x");
    let f = move || drop(s);
    run(f);
}
