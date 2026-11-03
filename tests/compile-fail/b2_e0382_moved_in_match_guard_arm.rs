//~ ERROR E0382
fn main() {
    let v = vec![String::from("a")];
    let n = 3;
    let r = match n {
        3 => v,
        _ => Vec::new(),
    };
    println!("{:?} {:?}", r, v);
}
