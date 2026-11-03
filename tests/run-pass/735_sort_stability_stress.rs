// Sorting stress: stable sort stability with many ties (checked against an
// insertion sort), sort_unstable/sort_by_cached_key/select_nth_unstable,
// partition_point, dedup_by_key, binary_search on unique keys, rotate/reverse,
// and sorting of strings / tuples / Reverse keys at different sizes.
use std::cmp::Reverse;

struct Xs(u64);
impl Xs {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13; x ^= x >> 7; x ^= x << 17;
        self.0 = x;
        x
    }
}

fn insertion_sort_by_key<T: Clone, K: Ord>(v: &[T], key: impl Fn(&T) -> K) -> Vec<T> {
    let mut out: Vec<T> = Vec::with_capacity(v.len());
    for x in v {
        let k = key(x);
        let pos = out.partition_point(|y| key(y) <= k);
        out.insert(pos, x.clone());
    }
    out
}

fn fp(v: &[(u32, u32)]) -> u64 {
    v.iter().fold(1469598103934665603u64, |h, &(a, b)| (h ^ (a as u64 * 65536 + b as u64)).wrapping_mul(1099511628211))
}

fn main() {
    let mut rng = Xs(88172645463325252);
    for &n in &[0usize, 1, 2, 7, 20, 21, 63, 100, 1000, 5000] {
        let data: Vec<(u32, u32)> = (0..n as u32).map(|i| ((rng.next() % 10) as u32, i)).collect();
        let mut s = data.clone();
        s.sort_by_key(|p| p.0);
        let expected = insertion_sort_by_key(&data, |p| p.0);
        let mut u = data.clone();
        u.sort_unstable();
        let mut rev = data.clone();
        rev.sort_by(|a, b| b.0.cmp(&a.0));
        let rev_ok = rev.windows(2).all(|w| w[0].0 > w[1].0 || (w[0].0 == w[1].0 && w[0].1 < w[1].1));
        let mut ck = data.clone();
        ck.sort_by_cached_key(|p| Reverse(p.0));
        let ck_ok = ck.windows(2).all(|w| w[0].0 > w[1].0 || (w[0].0 == w[1].0 && w[0].1 < w[1].1));
        println!("n={:<5} stable_ok={} unstable_sorted={} rev_stable={} cached_stable={} fp={:016x}", n, s == expected, u.windows(2).all(|w| w[0] <= w[1]), rev_ok, ck_ok, fp(&s));
    }

    let mut big: Vec<u64> = (0..20000).map(|_| rng.next() % 100000).collect();
    let mut copy = big.clone();
    copy.sort();
    let k = 777;
    let (_, nth, _) = big.select_nth_unstable(k);
    let nth = *nth;
    println!("select_nth {} == sorted[k] {}: {}", nth, copy[k], nth == copy[k]);
    let pp = copy.partition_point(|&x| x < 50000);
    println!("partition_point < 50000 at {} (checks {} {})", pp, copy[pp - 1] < 50000, copy[pp] >= 50000);
    let before = copy.len();
    copy.dedup();
    println!("dedup removed {}", before - copy.len());
    let probe: Vec<Result<usize, usize>> = [copy[0], copy[100], copy[copy.len() - 1], 100001].iter().map(|x| copy.binary_search(x)).collect();
    println!("binary_search {:?}", probe);
    big.sort_unstable_by(|a, b| b.cmp(a));
    println!("desc head {:?} tail {:?}", &big[..3], &big[big.len() - 3..]);

    let mut words: Vec<String> = "the Quick brown fox Jumps over the lazy dog and THE end of it all".split(' ').map(String::from).collect();
    words.sort();
    println!("{:?}", words);
    words.sort_by_key(|w| w.to_lowercase());
    println!("{:?}", words);
    words.sort_by(|a, b| a.len().cmp(&b.len()).then_with(|| b.cmp(a)));
    println!("{:?}", words);
    words.dedup_by_key(|w| w.to_lowercase());
    println!("dedup_by_key {:?}", words);
    words.dedup_by(|a, b| a.len() == b.len());
    println!("dedup_by len {:?}", words);

    let mut v: Vec<i32> = (1..=10).collect();
    v.rotate_left(3);
    print!("{:?} ", v);
    v.rotate_right(5);
    print!("{:?} ", v);
    v[2..8].reverse();
    println!("{:?}", v);
    let mut f = vec![3.5, -1.0, 2.25, f64::INFINITY, -0.0, 0.0];
    f.sort_by(|a, b| a.partial_cmp(b).unwrap());
    println!("floats {:?}", f);
    let mut tuples = vec![(2, "b", 1.5), (1, "z", 0.5), (2, "a", 9.0), (1, "z", -3.0)];
    tuples.sort_by(|a, b| (a.0, a.1).cmp(&(b.0, b.1)).then(a.2.partial_cmp(&b.2).unwrap()));
    println!("tuples {:?}", tuples);
    let mut chars: Vec<char> = "hello, wörld!".chars().collect();
    chars.sort_unstable();
    chars.dedup();
    println!("chars {}", chars.iter().collect::<String>());
    println!("is_sorted {} contains {} starts {} ends {}", v.is_sorted(), v.contains(&7), v.starts_with(&[4, 5]), v.ends_with(&[3]));
}
