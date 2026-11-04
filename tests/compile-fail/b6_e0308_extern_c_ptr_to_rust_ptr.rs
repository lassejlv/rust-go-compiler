//~ ERROR E0308
// Fn pointers of different ABIs are different types.
extern "C" fn c_inc(x: i32) -> i32 { x + 1 }

fn apply(f: fn(i32) -> i32, x: i32) -> i32 { f(x) }

fn main() {
    let p: extern "C" fn(i32) -> i32 = c_inc;
    println!("{}", apply(p, 1));
}
