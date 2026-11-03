//~ ERROR E0597
fn store<'a, T>(v: &mut Vec<&'a T>, x: &'a T) { v.push(x); }
fn main() {
    let mut v: Vec<&i32> = Vec::new();
    {
        let n = 3;
        store(&mut v, &n);
    }
    println!("{:?}", v);
}
