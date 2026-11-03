// Slice patterns binding disjoint &mut elements.
fn rotate3(v: &mut [i32]) {
    if let [a, b, c, ..] = v {
        let t = *a;
        *a = *b;
        *b = *c;
        *c = t;
    }
}

fn bump_ends(v: &mut [i32]) {
    match v {
        [] => {}
        [only] => *only += 1,
        [first, middle @ .., last] => {
            *first += 100;
            *last += 100;
            for m in middle.iter_mut() {
                *m += 1;
            }
        }
    }
}

fn main() {
    let mut v = vec![1, 2, 3, 4, 5];
    rotate3(&mut v);
    bump_ends(&mut v);
    let mut one = [7];
    bump_ends(&mut one);
    println!("{:?} {:?}", v, one);
}
