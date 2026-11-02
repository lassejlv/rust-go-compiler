fn sum_slice(s: &[i32]) -> i32 {
    let mut total = 0;
    for x in s {
        total += *x;
    }
    total
}

fn largest(s: &[i32]) -> i32 {
    let mut m = s[0];
    for &x in s {
        if x > m {
            m = x;
        }
    }
    m
}

fn double_all(s: &mut [i32]) {
    for x in s.iter_mut() {
        *x *= 2;
    }
}

fn main() {
    let a = [10, 20, 30, 40, 50, 60];
    let s = &a[1..4];
    println!("s = {:?}, len = {}", s, s.len());
    println!("sum(s) = {}", sum_slice(s));
    println!("sum(all) = {}", sum_slice(&a));
    println!("&a[..2] = {:?}", &a[..2]);
    println!("&a[4..] = {:?}", &a[4..]);
    println!("&a[..] = {:?}", &a[..]);
    println!("&a[2..=3] = {:?}", &a[2..=3]);
    println!("largest = {}", largest(&[3, 9, 2, 7]));
    let mut v = [1, 2, 3, 4, 5];
    double_all(&mut v[1..4]);
    println!("v = {:?}", v);
    let empty: &[i32] = &a[3..3];
    println!("empty len = {}, is_empty = {}", empty.len(), empty.is_empty());
    println!("first = {:?}, last = {:?}", a.first(), a.last());
    println!("get(2) = {:?}, get(10) = {:?}", a.get(2), a.get(10));
    let (left, right) = a.split_at(2);
    println!("split_at: {:?} {:?}", left, right);
    let total: i32 = s.iter().sum();
    println!("iter sum = {}", total);
    let words = ["a", "b", "c", "d"];
    let mid = &words[1..3];
    for w in mid.iter() {
        print!("[{}]", w);
    }
    println!();
    println!("starts_with: {}", a.starts_with(&[10, 20]));
    let mut arr2 = [5, 4, 3, 2, 1];
    arr2[1..4].reverse();
    println!("partial reverse: {:?}", arr2);
}
