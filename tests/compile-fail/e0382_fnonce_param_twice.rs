//~ ERROR E0382
fn call_twice<F: FnOnce() -> i32>(f: F) -> i32 {
    f() + f()
}

fn main() {
    println!("{}", call_twice(|| 1));
}
