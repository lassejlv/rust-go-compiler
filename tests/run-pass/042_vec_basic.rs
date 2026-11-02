fn main() {
    let mut v: Vec<i32> = Vec::new();
    println!("new: {:?} len={} empty={}", v, v.len(), v.is_empty());
    v.push(10);
    v.push(20);
    v.push(30);
    println!("after push: {:?} len={}", v, v.len());
    println!("v[1] = {}", v[1]);
    v[1] = 25;
    println!("after set: {:?}", v);
    println!("pop = {:?}", v.pop());
    println!("after pop: {:?}", v);
    v.insert(0, 5);
    v.insert(2, 15);
    println!("after insert: {:?}", v);
    let r = v.remove(1);
    println!("removed {} -> {:?}", r, v);
    println!("first = {:?}, last = {:?}", v.first(), v.last());
    println!("get(1) = {:?}, get(9) = {:?}", v.get(1), v.get(9));
    println!("contains 25: {}, contains 99: {}", v.contains(&25), v.contains(&99));
    let w = vec![1, 2, 3];
    let z = vec![0; 5];
    let s = vec!["a"; 3];
    println!("w={:?} z={:?} s={:?}", w, z, s);
    let mut total = 0;
    for x in &w {
        total += x;
    }
    println!("total = {}", total);
    let mut squares = Vec::with_capacity(5);
    for i in 0..5 {
        squares.push(i * i);
    }
    println!("squares = {:?}", squares);
    if let Some(last) = squares.last_mut() {
        *last = -1;
    }
    if let Some(first) = squares.first_mut() {
        *first = 100;
    }
    println!("modified = {:?}", squares);
    let removed = squares.swap_remove(1);
    println!("swap_remove {} -> {:?}", removed, squares);
    squares.clear();
    println!("cleared len = {}", squares.len());
    let mut empty: Vec<String> = vec![];
    println!("pop empty = {:?}", empty.pop());
    empty.push("x".to_string());
    println!("eq = {}", empty == vec!["x".to_string()]);
    let nested = vec![vec![1], vec![2, 3], vec![]];
    println!("nested = {:?} len = {}", nested, nested.len());
    let from_slice = [7, 8, 9].to_vec();
    let from_iter: Vec<u32> = (1..=4).collect();
    println!("to_vec {:?}, collect {:?}", from_slice, from_iter);
}
