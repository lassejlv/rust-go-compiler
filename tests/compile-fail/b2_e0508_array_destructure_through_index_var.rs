//~ ERROR E0508
fn main() {
    let arr = [vec![1], vec![2]];
    let i = 1;
    let v = arr[i];
    println!("{:?}", v);
}
