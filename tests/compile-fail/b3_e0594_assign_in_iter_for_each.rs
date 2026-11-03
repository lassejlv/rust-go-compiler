//~ ERROR E0594
fn main() {
    let v = vec![1, 2];
    v.iter().for_each(|x| *x = 0);
}
