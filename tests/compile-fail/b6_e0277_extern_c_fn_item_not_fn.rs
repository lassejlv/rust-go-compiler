//~ ERROR E0277
// `extern "C"` fn items do not implement the `Fn` traits.
extern "C" fn c_square(x: i32) -> i32 { x * x }

fn call<F: Fn(i32) -> i32>(f: F) -> i32 { f(3) }

fn main() {
    println!("{}", call(c_square));
}
