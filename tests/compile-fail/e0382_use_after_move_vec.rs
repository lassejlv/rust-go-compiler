//~ ERROR E0382
fn main() {
    let v = vec![1, 2, 3];
    let w = v;
    println!("{:?} {:?}", v, w);
}
