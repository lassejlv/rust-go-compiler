//~ ERROR E0500
fn main() {
    let mut v = vec![1];
    let x = &mut v;
    let r = &x;
    let mut f = || x.push(2);
    f();
    println!("{:?}", r);
}
