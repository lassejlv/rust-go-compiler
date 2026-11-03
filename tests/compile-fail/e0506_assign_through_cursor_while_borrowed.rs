//~ ERROR E0506
fn main() {
    let mut arr = [1, 2, 3];
    let slice = &arr[..];
    arr[0] = 9;
    println!("{:?}", slice);
}
