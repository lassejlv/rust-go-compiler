//~ ERROR E0382
fn main() {
    let data = vec![1, 2, 3];
    let f = move || data.len();
    println!("{}", f());
    println!("{:?}", data);
}
