//~ ERROR E0382
fn main() {
    let v = vec![1];
    let w = v;
    let r = match w.len() {
        0 => 0,
        _ => v.len(),
    };
    println!("{}", r);
}
