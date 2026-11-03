//~ ERROR E0382
fn main() {
    let o: Option<Vec<u8>> = Some(vec![1]);
    let v = o.unwrap_or_default();
    let w = o.map(|x| x.len());
    println!("{:?} {:?}", v, w);
}
