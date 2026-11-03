//~ ERROR E0308
fn apply<F: Fn(i32) -> i32>(f: F) -> i32 {
    f(1)
}

fn main() {
    println!("{}", apply(|x| x > 0));
}
