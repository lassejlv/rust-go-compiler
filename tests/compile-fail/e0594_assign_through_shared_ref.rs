//~ ERROR E0594
fn set(r: &i32) {
    *r = 5;
}

fn main() {
    let mut x = 1;
    set(&x);
    println!("{}", x);
}
