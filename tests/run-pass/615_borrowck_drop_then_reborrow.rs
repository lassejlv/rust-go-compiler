// Dropping a borrow (explicitly or by scope) and then borrowing again.
fn main() {
    let mut v = vec![1, 2, 3];
    let r = &mut v;
    r.push(4);
    let s = &v;
    println!("{:?}", s);
    {
        let m = &mut v;
        m.retain(|x| x % 2 == 0);
    }
    let first = v.first().copied();
    v.push(10);
    let it = v.iter();
    let total: i32 = it.sum();
    v.clear();
    v.push(total);
    println!("{:?} {:?}", v, first);
}
