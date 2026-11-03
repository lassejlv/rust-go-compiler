//~ ERROR E0597
fn main() {
    let refs: Vec<&i32>;
    {
        let v = vec![1, 2, 3];
        refs = v.iter().filter(|x| **x > 1).collect();
    }
    println!("{:?}", refs);
}
