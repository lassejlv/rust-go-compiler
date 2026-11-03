//~ ERROR E0505
fn main() {
    let v = vec![1];
    let r = &v;
    let h = std::thread::spawn(move || v.len());
    println!("{:?}", r);
    h.join().unwrap();
}
