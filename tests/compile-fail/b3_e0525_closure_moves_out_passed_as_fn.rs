//~ ERROR E0525
fn call_twice<F: Fn() -> String>(f: F) -> String { f() + &f() }
fn main() {
    let s = String::from("x");
    let f = move || s;
    call_twice(f);
}
