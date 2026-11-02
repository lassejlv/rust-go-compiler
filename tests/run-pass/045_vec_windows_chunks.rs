fn main() {
    let v = vec![1, 3, 6, 10, 15, 21, 28];
    let diffs: Vec<i32> = v.windows(2).map(|w| w[1] - w[0]).collect();
    println!("diffs: {:?}", diffs);
    for w in v.windows(3) {
        print!("{:?} ", w);
    }
    println!();
    for c in v.chunks(3) {
        print!("{:?} ", c);
    }
    println!();
    let sums: Vec<i32> = v.chunks(2).map(|c| c.iter().sum()).collect();
    println!("chunk sums: {:?}", sums);
    for c in v.chunks_exact(3) {
        print!("exact{:?} ", c);
    }
    println!();
    for c in v.rchunks(3) {
        print!("r{:?} ", c);
    }
    println!();
    let words = vec!["alpha", "beta", "gamma"];
    println!("join: {}", words.join("-"));
    println!("concat: {}", words.concat());
    let vv = vec![vec![1, 2], vec![3], vec![4, 5, 6]];
    println!("concat vecs: {:?}", vv.concat());
    let joined: Vec<i32> = vec![vec![1, 2], vec![3, 4]].join(&0);
    println!("join vecs with 0: {:?}", joined);
    if let Some((first, rest)) = v.split_first() {
        println!("split_first: {} {:?}", first, rest);
    }
    if let Some((last, rest)) = v.split_last() {
        println!("split_last: {} {:?}", last, rest);
    }
    let parts: Vec<&[i32]> = [1, 0, 2, 3, 0, 4].split(|&x| x == 0).collect();
    println!("split on 0: {:?}", parts);
    println!("position of 10: {:?}", v.iter().position(|&x| x == 10));
    println!("rposition >5: {:?}", v.iter().rposition(|&x| x > 5));
    println!("max {:?} min {:?}", v.iter().max(), v.iter().min());
    println!("contains 21: {}", v.contains(&21));
    println!("ends_with: {}", v.ends_with(&[21, 28]));
    let moving_avg: Vec<f64> = v.windows(3).map(|w| w.iter().sum::<i32>() as f64 / 3.0).collect();
    println!("moving avg: {:?}", moving_avg);
    let pairs: Vec<(i32, i32)> = v.chunks(2).filter(|c| c.len() == 2).map(|c| (c[0], c[1])).collect();
    println!("pairs: {:?}", pairs);
    let repeated = [1, 2].repeat(3);
    println!("repeat: {:?}", repeated);
    let mut m = vec![4, 5, 6];
    m.copy_from_slice(&[7, 8, 9]);
    println!("copy_from_slice: {:?}", m);
    let iter_rev: Vec<&i32> = v.iter().rev().take(3).collect();
    println!("last three reversed: {:?}", iter_rev);
}
