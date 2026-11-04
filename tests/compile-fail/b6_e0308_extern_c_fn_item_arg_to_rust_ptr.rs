//~ ERROR E0308
// An `extern "C"` fn item passed where a Rust fn pointer is expected.
extern "C" fn c_inc(x: i32) -> i32 { x + 1 }

fn apply(f: fn(i32) -> i32, x: i32) -> i32 { f(x) }

fn main() {
    println!("{}", apply(c_inc, 1));
}
