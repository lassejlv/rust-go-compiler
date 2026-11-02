fn main() {
    let v = [30, 20, 10];
    println!("{:?}", v.binary_search_by(|p| 20.cmp(p)));
    let x: &i32 = &5; println!("{:?} {:?}", 3.cmp(x), 7.max(2));
}
