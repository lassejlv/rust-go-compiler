// Long iterator adaptor chains: filter_map, flat_map, flatten, zip, chain, rev, step_by,
// enumerate, skip, take, inspect, map_while, cycle+take, sum/product/min_by/max_by_key.
fn main() {
    let words = ["apple", "Banana", "cherry", "date", "Elderberry", "fig", "grape"];

    let r: Vec<String> = words
        .iter()
        .enumerate()
        .filter(|(i, _)| i % 2 == 0)
        .map(|(i, w)| (i, w.to_lowercase()))
        .filter_map(|(i, w)| if w.len() > 3 { Some(format!("{}{}", i, &w[..3])) } else { None })
        .rev()
        .collect();
    println!("{:?}", r);

    let letters: String = words.iter().flat_map(|w| w.chars().take(2)).filter(|c| c.is_lowercase()).collect();
    println!("{}", letters);

    let nested = vec![vec![1, 2], vec![], vec![3], vec![4, 5, 6]];
    let flat: Vec<i32> = nested.iter().flatten().copied().collect();
    let flat2: Vec<i32> = nested.into_iter().flatten().rev().step_by(2).collect();
    println!("{:?} {:?}", flat, flat2);

    let a = [1, 2, 3, 4, 5];
    let b = ["one", "two", "three"];
    let zipped: Vec<String> = a.iter().zip(b.iter()).map(|(n, s)| format!("{}={}", n, s)).collect();
    println!("{}", zipped.join(","));

    let chained: Vec<i32> = (1..4).chain(10..12).chain(std::iter::once(100)).chain(std::iter::repeat(7).take(2)).collect();
    println!("{:?}", chained);

    let stepped: Vec<u32> = (0..50).step_by(7).skip(1).take(4).collect();
    println!("{:?}", stepped);
    let rstep: Vec<i32> = (0..=20).rev().step_by(5).collect();
    println!("{:?}", rstep);

    let mut inspected = Vec::new();
    let total: i32 = (1..=6).inspect(|x| inspected.push(*x)).filter(|x| x % 3 != 0).map(|x| x * x).sum();
    println!("total={} inspected={:?}", total, inspected);

    let mw: Vec<i32> = [4, 8, 15, 16, 23, 42].iter().map_while(|&x| if x < 20 { Some(x / 2) } else { None }).collect();
    println!("map_while {:?}", mw);

    let cyc: String = "abc".chars().cycle().skip(1).take(7).collect();
    println!("cycle {}", cyc);

    let prod: u64 = (1..=10u64).filter(|n| n % 2 == 1).product();
    println!("product of odds {}", prod);

    let longest = words.iter().max_by_key(|w| w.len()).unwrap();
    let shortest = words.iter().min_by_key(|w| w.len()).unwrap();
    let last_alpha = words.iter().max_by(|a, b| a.to_lowercase().cmp(&b.to_lowercase())).unwrap();
    println!("{} {} {}", longest, shortest, last_alpha);

    let firsts: Vec<(char, usize)> = words
        .iter()
        .map(|w| (w.chars().next().unwrap().to_ascii_uppercase(), w.len()))
        .filter(|&(c, _)| c != 'D')
        .skip_while(|&(_, l)| l > 4)
        .collect();
    println!("{:?}", firsts);

    let pos = words.iter().position(|w| w.starts_with('c'));
    let any_upper = words.iter().any(|w| w.chars().all(|c| c.is_uppercase()));
    let all_ascii = words.iter().all(|w| w.is_ascii());
    let cnt = words.iter().filter(|w| w.contains('e')).count();
    println!("{:?} {} {} {}", pos, any_upper, all_ascii, cnt);

    let last = (1..100).filter(|x| x % 13 == 0).last();
    let nth = (1..100).filter(|x| x % 13 == 0).nth(3);
    let found = words.iter().find(|w| w.len() == 4).map(|w| w.to_uppercase());
    let fm = words.iter().find_map(|w| w.strip_prefix("gr"));
    println!("{:?} {:?} {:?} {:?}", last, nth, found, fm);

    let (small, big): (Vec<i32>, Vec<i32>) = (1..=10).partition(|x| *x <= 3);
    println!("{:?} {:?}", small, big);
    let (xs, ys): (Vec<i32>, Vec<char>) = vec![(1, 'a'), (2, 'b'), (3, 'c')].into_iter().unzip();
    println!("{:?} {:?}", xs, ys);

    let cmp = [1, 2, 3].iter().cmp([1, 2, 4].iter());
    let eq = (1..4).eq([1, 2, 3].iter().copied());
    let lt = [1.0, 2.0].iter().partial_cmp([1.0, f64::NAN].iter());
    println!("{:?} {} {:?}", cmp, eq, lt);

    let sum_sq: i64 = (1..=100i64).rev().filter(|n| n % 7 == 0).map(|n| n * n).take(3).sum();
    println!("sum_sq={}", sum_sq);
}
