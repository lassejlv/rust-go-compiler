//~ ERROR E0616
fn main() {
    let v = vec![1];
    let n: usize = v.len;
    println!("{}", n);
}
