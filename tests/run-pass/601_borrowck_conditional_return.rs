// Conditionally returning a borrow (accepted by NLL when the borrow is not
// live in the fallthrough path through a different borrow of the same map).
use std::collections::HashMap;

fn get_or_insert(map: &mut HashMap<u32, String>, key: u32) -> &mut String {
    if !map.contains_key(&key) {
        map.insert(key, format!("v{}", key));
    }
    map.get_mut(&key).unwrap()
}

fn first_positive(v: &mut Vec<i32>) -> &mut i32 {
    if let Some(i) = v.iter().position(|x| *x > 0) {
        return &mut v[i];
    }
    v.push(1);
    v.last_mut().unwrap()
}

fn bigger<'a>(a: &'a mut Vec<i32>, b: &'a mut Vec<i32>) -> &'a mut Vec<i32> {
    if a.len() >= b.len() {
        return a;
    }
    b.push(0);
    b
}

fn main() {
    let mut m = HashMap::new();
    get_or_insert(&mut m, 1).push('!');
    get_or_insert(&mut m, 1).push('?');
    println!("{}", m[&1]);
    let mut v = vec![-1, -2, 3, 4];
    *first_positive(&mut v) *= 10;
    let mut w = vec![-1];
    *first_positive(&mut w) += 5;
    bigger(&mut v, &mut w).push(99);
    bigger(&mut w, &mut vec![1, 2, 3]).push(7);
    println!("{:?} {:?}", v, w);
}
