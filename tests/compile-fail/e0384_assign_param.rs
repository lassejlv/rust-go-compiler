//~ ERROR E0384
fn inc(n: i32) -> i32 {
    n += 1;
    n
}

fn main() {
    println!("{}", inc(1));
}
