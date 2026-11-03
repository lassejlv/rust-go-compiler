//~ ERROR E0597
fn main() {
    let it;
    {
        let v = vec![1, 2, 3];
        it = v.iter().map(|x| x * 2);
    }
    let s: i32 = it.sum();
    println!("{}", s);
}
