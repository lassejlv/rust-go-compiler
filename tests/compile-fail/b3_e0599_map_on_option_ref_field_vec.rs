//~ ERROR E0599
struct S { v: Vec<i32> }
fn main() {
    let s = S { v: vec![1] };
    let d: Vec<i32> = s.v.map(|x| x * 2).collect();
    println!("{:?}", d);
}
