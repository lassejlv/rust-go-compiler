// split_at_mut / chunks_mut / swap to get two &mut into one slice.
fn main() {
    let mut a = [1, 2, 3, 4, 5, 6];
    {
        let (left, right) = a.split_at_mut(3);
        left[0] += right[0];
        right[2] += left[2];
        std::mem::swap(&mut left[1], &mut right[1]);
    }
    println!("{:?}", a);
    for chunk in a.chunks_mut(2) {
        let (x, y) = chunk.split_at_mut(1);
        x[0] += y[0];
    }
    println!("{:?}", a);
    a.swap(0, 5);
    if let [first, .., last] = &mut a[..] {
        std::mem::swap(first, last);
        *first *= 100;
    }
    println!("{:?}", a);
}
