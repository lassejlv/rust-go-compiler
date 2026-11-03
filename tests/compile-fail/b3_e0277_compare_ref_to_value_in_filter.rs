//~ ERROR E0277
fn main() {
    let v = vec![1, 2, 3];
    let n = v.iter().filter(|x| x == 2).count();
    println!("{}", n);
}
