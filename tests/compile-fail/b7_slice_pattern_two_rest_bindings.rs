//~ ERROR `..` can only be used once per slice pattern
// Also when one of them is bound with `@`.
fn main() {
    let v = vec![1, 2, 3];
    if let [head @ .., _, tail @ ..] = v.as_slice() {
        println!("{:?} {:?}", head, tail);
    }
}
