//~ ERROR E0015
// `vec![..]` allocates: not allowed in a static.
static V: Vec<i32> = vec![1, 2, 3];
fn main() {
    println!("{:?}", V);
}
