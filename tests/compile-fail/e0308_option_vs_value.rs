//~ ERROR E0308
fn find(v: &[i32]) -> i32 {
    v.iter().copied().find(|x| *x > 1)
}

fn main() {
    println!("{}", find(&[1, 2]));
}
