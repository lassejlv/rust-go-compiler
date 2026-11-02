fn median(v: &[i32]) -> f64 {
    let mut s = v.to_vec();
    s.sort();
    let n = s.len();
    if n % 2 == 0 {
        (s[n / 2 - 1] + s[n / 2]) as f64 / 2.0
    } else {
        s[n / 2] as f64
    }
}

fn mode(v: &[i32]) -> i32 {
    let mut s = v.to_vec();
    s.sort();
    let mut best = s[0];
    let mut best_count = 0;
    let mut i = 0;
    while i < s.len() {
        let mut j = i;
        while j < s.len() && s[j] == s[i] {
            j += 1;
        }
        if j - i > best_count {
            best_count = j - i;
            best = s[i];
        }
        i = j;
    }
    best
}

fn histogram(v: &[i32], buckets: i32, lo: i32, hi: i32) -> Vec<usize> {
    let mut h = vec![0; buckets as usize];
    let width = (hi - lo) / buckets;
    for &x in v {
        let mut b = (x - lo) / width;
        if b >= buckets {
            b = buckets - 1;
        }
        h[b as usize] += 1;
    }
    h
}

fn main() {
    let data = vec![23, 7, 42, 7, 15, 99, 64, 7, 33, 42, 81, 5, 56, 18, 90];
    println!("data: {:?}", data);
    println!("len: {}", data.len());
    println!("sum: {}", data.iter().sum::<i32>());
    println!("min: {}, max: {}", data.iter().min().unwrap(), data.iter().max().unwrap());
    println!("mean: {:.3}", data.iter().sum::<i32>() as f64 / data.len() as f64);
    println!("median: {}", median(&data));
    println!("median even: {}", median(&[4, 1, 3, 2]));
    println!("mode: {}", mode(&data));
    let h = histogram(&data, 5, 0, 100);
    for (i, count) in h.iter().enumerate() {
        println!("{:>3}-{:<3} {}", i * 20, i * 20 + 19, "*".repeat(*count));
    }
    let mut sorted = data.clone();
    sorted.sort_unstable_by(|a, b| b.cmp(a));
    println!("top 3: {:?}", &sorted[..3]);
    let evens: Vec<i32> = data.iter().filter(|x| *x % 2 == 0).cloned().collect();
    println!("evens: {:?}", evens);
    let above_mean: usize = {
        let m = data.iter().sum::<i32>() / data.len() as i32;
        data.iter().filter(|&&x| x > m).count()
    };
    println!("above mean: {}", above_mean);
    let running: Vec<i32> = data.iter().scan(0, |acc, &x| { *acc += x; Some(*acc) }).collect();
    println!("running sum: {:?}", running);
    let (mn, mx) = data.iter().fold((i32::MAX, i32::MIN), |(mn, mx), &x| (mn.min(x), mx.max(x)));
    println!("range: {}..{} (span {})", mn, mx, mx - mn);
}
