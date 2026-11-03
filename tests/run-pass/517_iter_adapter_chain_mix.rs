// long iterator adapter chains mixing many adapters
fn main() {
    let a = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
    let b = ["one", "two", "three", "four", "five"];

    let r: Vec<String> = a
        .iter()
        .zip(b.iter().cycle())
        .filter(|(n, _)| *n % 3 != 0)
        .map(|(n, s)| format!("{}{}", n, &s[..1]))
        .skip(1)
        .step_by(2)
        .collect();
    println!("{:?}", r);

    let s: i32 = a
        .iter()
        .chain([100, 200].iter())
        .rev()
        .skip_while(|x| **x > 50)
        .take_while(|x| **x > 2)
        .scan(0, |acc, &x| {
            *acc += x;
            Some(*acc)
        })
        .inspect(|v| print!("[{}]", v))
        .filter(|v| v % 2 == 1)
        .sum();
    println!(" sum_odd_prefix={}", s);

    let fm: Vec<(usize, char)> = b
        .iter()
        .enumerate()
        .flat_map(|(i, w)| w.chars().filter(|c| "aeiou".contains(*c)).map(move |c| (i, c)))
        .collect();
    println!("{:?}", fm);

    let flat: Vec<i32> = vec![vec![1, 2], vec![], vec![3], vec![4, 5, 6]].into_iter().flatten().rev().collect();
    println!("{:?}", flat);

    let mut it = a.iter().map(|x| x * 10).fuse();
    let firsts: Vec<_> = it.by_ref().take(3).collect();
    let rest_sum: i32 = it.sum();
    println!("{:?} {}", firsts, rest_sum);

    let windows: Vec<i32> = a.windows(3).map(|w| w.iter().product()).filter(|p: &i32| p % 4 == 0).collect();
    println!("{:?}", windows);
    let ch: Vec<Vec<i32>> = a.chunks(4).map(|c| c.iter().rev().cloned().collect()).collect();
    println!("{:?}", ch);
    let ce = a.chunks_exact(3);
    let rem = ce.remainder().to_vec();
    let sums: Vec<i32> = ce.map(|c| c.iter().sum()).collect();
    println!("{:?} rem={:?}", sums, rem);
    let rch: Vec<&[i32]> = a.rchunks(3).collect();
    println!("{:?}", rch);

    let pos = a.iter().rposition(|x| x % 4 == 0);
    let mx = b.iter().max_by_key(|w| w.len());
    let mn = b.iter().min_by(|x, y| x.len().cmp(&y.len()).then_with(|| y.cmp(x)));
    println!("{:?} {:?} {:?}", pos, mx, mn);
    let lst = b.iter().rev().skip(1).last();
    println!("{:?} {:?}", lst, b.iter().nth_back(1));

    let (small, big): (Vec<i32>, Vec<i32>) = a.iter().partition(|&&x| x < 4);
    println!("{:?} {:?}", small, big);
    let cmp = [1, 2, 3].iter().cmp([1, 2, 4].iter());
    let pcmp = [1.0, f64::NAN].iter().partial_cmp([1.0, 2.0].iter());
    println!("{:?} {:?} {}", cmp, pcmp, [1, 2].iter().eq([1, 2].iter()));
    let total_len = b.iter().map(|s| s.len()).fold(String::new(), |mut acc, l| {
        if !acc.is_empty() {
            acc.push('+');
        }
        acc += &l.to_string();
        acc
    });
    println!("{}", total_len);
    let tf: Option<i32> = a.iter().try_fold(0i32, |acc, &x| acc.checked_add(x * 100_000_000));
    println!("{:?}", tf);
    let found = a.iter().find_map(|&x| if x * x > 30 { Some(x * 1000) } else { None });
    println!("{:?}", found);
    let mut pk = a.iter().peekable();
    let mut groups = vec![];
    while let Some(&x) = pk.next() {
        let mut g = vec![x];
        while let Some(&&y) = pk.peek() {
            if y / 4 == x / 4 {
                g.push(y);
                pk.next();
            } else {
                break;
            }
        }
        groups.push(g);
    }
    println!("{:?}", groups);
    let nested_sum: u64 = (1..=4u64).map(|i| (1..=i).map(|j| j * i).product::<u64>()).sum();
    println!("{}", nested_sum);
    let maxf = [2.5f64, -1.0, 9.75, 3.0].iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    println!("{}", maxf);
    let s: String = "hello world".split(' ').map(|w| w.chars().rev().collect::<String>()).collect::<Vec<_>>().join(" ");
    println!("{}", s);
    let cnt = (0..100).filter(|x| x % 7 == 0).map(|x| x * 2).take_while(|&x| x < 150).count();
    println!("{}", cnt);
    let mut sorted: Vec<_> = b.iter().map(|s| (s.len(), *s)).collect();
    sorted.sort_unstable();
    sorted.dedup_by_key(|p| p.0);
    println!("{:?}", sorted);
    println!("{}", (1..=5).map(|i| i.to_string()).rev().collect::<Vec<_>>().concat());
    println!("{:?}", "a-b_c d".split(['-', '_', ' ']).collect::<Vec<_>>());
    println!("{}", a.iter().any(|&x| x > 9) && a.iter().all(|&x| x > 0));
}
