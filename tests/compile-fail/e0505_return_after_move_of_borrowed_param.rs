//~ ERROR E0505
fn pass_through(v: Vec<i32>) -> Vec<i32> {
    v
}

fn main() {
    let v = vec![1, 2];
    let s = &v[..];
    let w = pass_through(v);
    println!("{:?} {:?}", s, w);
}
