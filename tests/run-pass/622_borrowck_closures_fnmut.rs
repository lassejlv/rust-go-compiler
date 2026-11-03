// FnMut closures mutably borrowing locals; borrow ends after last call.
fn apply_n<F: FnMut(i32)>(n: i32, mut f: F) {
    for i in 0..n {
        f(i);
    }
}

fn main() {
    let mut acc = Vec::new();
    apply_n(4, |i| acc.push(i * i));
    acc.push(-1);
    let mut sum = 0;
    let mut add = |x: i32| sum += x;
    add(1);
    add(2);
    println!("{} {:?}", sum, acc);
    let mut counter = 0;
    let mut next = || {
        counter += 1;
        counter
    };
    let a = next();
    let b = next();
    counter += 100;
    let mut v = vec![3, 1, 2];
    v.sort_by_key(|x| {
        counter += 1;
        *x
    });
    println!("{} {} {} {:?}", a, b, counter, v);
}
