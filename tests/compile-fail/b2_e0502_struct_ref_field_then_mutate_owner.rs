//~ ERROR E0502
struct View<'a> { slice: &'a [i32] }
fn main() {
    let mut data = vec![1, 2, 3];
    let view = View { slice: &data[..] };
    data.push(4);
    println!("{}", view.slice.len());
}
