//~ ERROR E0277
// `extern "C"` fn pointers do not implement `Fn` either.
extern "C" fn c_neg(x: i32) -> i32 { -x }

fn apply_all<F: Fn(i32) -> i32>(xs: &[i32], f: F) -> Vec<i32> {
    xs.iter().map(|&x| f(x)).collect()
}

fn main() {
    let p: extern "C" fn(i32) -> i32 = c_neg;
    println!("{:?}", apply_all(&[1, 2], p));
}
