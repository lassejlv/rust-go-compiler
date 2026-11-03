//~ ERROR E0505
fn consume(_v: Vec<i32>) {}
fn main() {
    let v = vec![1, 2];
    let it = v.iter();
    consume(v);
    for x in it { println!("{}", x); }
}
