// Shadowing a borrowed variable and rebinding references.
fn main() {
    let s = String::from("hello");
    let r = &s;
    let s = r.to_uppercase();
    println!("{} {}", r, s);
    let mut a = 1;
    let mut b = 2;
    let mut p = &mut a;
    *p += 10;
    p = &mut b;
    *p += 20;
    println!("{} {}", a, b);
    let mut x = vec![1];
    let mut y = vec![2];
    let mut cur = &mut x;
    for i in 0..4 {
        cur.push(i);
        cur = if i % 2 == 0 { &mut y } else { &mut x };
    }
    println!("{:?} {:?}", x, y);
}
