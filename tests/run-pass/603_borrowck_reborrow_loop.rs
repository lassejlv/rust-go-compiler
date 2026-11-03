// Reborrowing &mut *x repeatedly in a loop.
fn bump(x: &mut i32, by: i32) {
    *x += by;
}

fn push_twice(v: &mut Vec<i32>, x: i32) {
    v.push(x);
    v.push(x);
}

fn work(x: &mut i32, v: &mut Vec<i32>) {
    for i in 0..5 {
        bump(&mut *x, i);
        bump(x, 1);
        push_twice(&mut *v, *x);
    }
    let r: &mut Vec<i32> = v;
    r.push(-1);
    v.push(-2);
}

fn main() {
    let mut x = 0;
    let mut v = Vec::new();
    work(&mut x, &mut v);
    println!("{} {:?}", x, v);
}
