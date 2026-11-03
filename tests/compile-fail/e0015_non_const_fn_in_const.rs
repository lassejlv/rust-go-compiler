//~ ERROR E0015
fn compute() -> u32 {
    42
}

const X: u32 = compute();

fn main() {
    println!("{}", X);
}
