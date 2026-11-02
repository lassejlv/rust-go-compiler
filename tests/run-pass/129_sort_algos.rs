fn quicksort(v: &mut [i32]) {
    if v.len() <= 1 {
        return;
    }
    let p = partition(v);
    let (left, right) = v.split_at_mut(p);
    quicksort(left);
    quicksort(&mut right[1..]);
}

fn partition(v: &mut [i32]) -> usize {
    let hi = v.len() - 1;
    let mid = hi / 2;
    v.swap(mid, hi);
    let pivot = v[hi];
    let mut i = 0;
    for j in 0..hi {
        if v[j] <= pivot {
            v.swap(i, j);
            i += 1;
        }
    }
    v.swap(i, hi);
    i
}

fn merge_sort(v: Vec<i32>) -> Vec<i32> {
    if v.len() <= 1 {
        return v;
    }
    let mid = v.len() / 2;
    let right = merge_sort(v[mid..].to_vec());
    let left = merge_sort(v[..mid].to_vec());
    let mut out = Vec::with_capacity(left.len() + right.len());
    let (mut i, mut j) = (0, 0);
    while i < left.len() && j < right.len() {
        if left[i] <= right[j] {
            out.push(left[i]);
            i += 1;
        } else {
            out.push(right[j]);
            j += 1;
        }
    }
    out.extend_from_slice(&left[i..]);
    out.extend_from_slice(&right[j..]);
    out
}

fn insertion_sort<T: PartialOrd + Copy>(v: &mut Vec<T>) -> usize {
    let mut shifts = 0;
    for i in 1..v.len() {
        let key = v[i];
        let mut j = i;
        while j > 0 && v[j - 1] > key {
            v[j] = v[j - 1];
            j -= 1;
            shifts += 1;
        }
        v[j] = key;
    }
    shifts
}

fn heap_sort(v: &mut [i32]) {
    fn sift(v: &mut [i32], mut root: usize, end: usize) {
        loop {
            let mut child = 2 * root + 1;
            if child >= end {
                break;
            }
            if child + 1 < end && v[child] < v[child + 1] {
                child += 1;
            }
            if v[root] >= v[child] {
                break;
            }
            v.swap(root, child);
            root = child;
        }
    }
    let n = v.len();
    for i in (0..n / 2).rev() {
        sift(v, i, n);
    }
    for end in (1..n).rev() {
        v.swap(0, end);
        sift(v, 0, end);
    }
}

fn counting_sort(v: &[u8]) -> Vec<u8> {
    let mut counts = [0usize; 256];
    for &x in v {
        counts[x as usize] += 1;
    }
    let mut out = Vec::new();
    for (val, &c) in counts.iter().enumerate() {
        for _ in 0..c {
            out.push(val as u8);
        }
    }
    out
}

fn pseudo_random(n: usize, seed: u64) -> Vec<i32> {
    let mut x = seed;
    (0..n)
        .map(|_| {
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            (x % 1000) as i32 - 500
        })
        .collect()
}

fn main() {
    let data = vec![38, 27, 43, 3, 9, 82, 10, 3, -5, 0, 27];
    let mut q = data.clone();
    quicksort(&mut q);
    println!("quicksort:  {:?}", q);
    println!("merge_sort: {:?}", merge_sort(data.clone()));
    let mut ins = data.clone();
    let shifts = insertion_sort(&mut ins);
    println!("insertion:  {:?} ({} shifts)", ins, shifts);
    let mut h = data.clone();
    heap_sort(&mut h);
    println!("heap_sort:  {:?}", h);
    println!("counting:   {:?}", counting_sort(&[5, 3, 200, 3, 0, 255, 17]));
    let mut fl = vec![3.5, -1.25, 2.0, 0.0];
    insertion_sort(&mut fl);
    println!("insertion f64: {:?}", fl);
    let big = pseudo_random(2000, 0x2545F4914F6CDD1D);
    println!("random sample: {:?}", &big[..8]);
    let mut expected = big.clone();
    expected.sort();
    let mut a = big.clone();
    quicksort(&mut a);
    let b = merge_sort(big.clone());
    let mut c = big.clone();
    heap_sort(&mut c);
    println!("all agree with std sort: {} {} {}", a == expected, b == expected, c == expected);
    println!("min {} max {} median {}", expected[0], expected[expected.len() - 1], expected[expected.len() / 2]);
    let checksum: i64 = expected.iter().enumerate().map(|(i, &x)| i as i64 * x as i64).sum();
    println!("checksum {}", checksum);
    let mut empty: Vec<i32> = vec![];
    quicksort(&mut empty);
    heap_sort(&mut empty);
    println!("empty ok: {:?} {:?}", empty, merge_sort(vec![]));
}
