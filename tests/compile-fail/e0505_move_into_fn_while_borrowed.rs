//~ ERROR E0505
fn eat(_v: Vec<i32>) {}

fn main() {
    let v = vec![1, 2];
    let first = &v[0];
    eat(v);
    println!("{}", first);
}
