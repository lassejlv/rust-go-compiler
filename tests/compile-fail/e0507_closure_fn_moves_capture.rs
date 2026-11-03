//~ ERROR E0507
fn call<F: Fn() -> String>(f: F) -> String {
    f()
}

fn main() {
    let s = String::from("captured");
    println!("{}", call(|| s));
}
