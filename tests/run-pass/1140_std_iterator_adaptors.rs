// Breadth test of iterator adaptors and consumers.
use std::iter;

fn main() {
    let a = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
    let words = ["apple", "Bob", "cat", "Dog", "egg", "fig"];

    let z: Vec<(i32, &str)> = a.iter().copied().zip(words).collect();
    println!("zip: {:?}", z);
    let z3: Vec<(i32, char, bool)> = iter::zip(a, "xyz".chars()).zip([true, false]).map(|((n, c), b)| (n, c, b)).collect();
    println!("iter::zip: {:?}", z3);
    let (nums, ws): (Vec<i32>, Vec<&str>) = z.iter().cloned().unzip();
    println!("unzip: {:?} {:?}", nums, ws);
    let (even, odd): (Vec<i32>, Vec<i32>) = a.iter().partition(|&&x| x % 2 == 0);
    println!("partition: {:?} {:?}", even, odd);
    println!("fold: {}", a.iter().fold(String::new(), |acc, x| format!("{}{}", acc, x)));
    println!("rfold: {}", a.iter().rfold(0i64, |acc, &x| acc * 2 + x as i64));
    let tf: Result<i32, String> = a.iter().try_fold(0i32, |acc, &x| if x < 8 { Ok(acc + x) } else { Err(format!("stop at {} (acc {})", x, acc)) });
    println!("try_fold: {:?}", tf);
    let tf2: Option<u8> = a.iter().try_fold(0u8, |acc, &x| acc.checked_add(x as u8 * 20));
    println!("try_fold overflow: {:?}", tf2);
    println!("scan: {:?}", a.iter().scan(1, |st, &x| { *st *= x; if *st > 5000 { None } else { Some(*st) } }).collect::<Vec<_>>());
    println!("flat_map: {:?}", words.iter().take(3).flat_map(|w| w.chars().rev()).collect::<String>());
    println!("flatten: {:?}", vec![vec![1, 2], vec![], vec![3]].into_iter().flatten().collect::<Vec<_>>());
    println!("flatten options: {:?}", [Some(1), None, Some(3)].into_iter().flatten().sum::<i32>());
    println!("step_by: {:?}", (0..20).step_by(7).collect::<Vec<_>>());
    println!("rev step_by: {:?}", (0..20).rev().step_by(6).collect::<Vec<_>>());
    println!("skip_while: {:?}", a.iter().skip_while(|&&x| x < 4).take(3).collect::<Vec<_>>());
    println!("take_while: {:?}", a.iter().take_while(|&&x| x * x < 30).collect::<Vec<_>>());
    println!("map_while: {:?}", ["1", "22", "x", "4"].iter().map_while(|s| s.parse::<i32>().ok()).collect::<Vec<_>>());
    let mut seen = vec![];
    let s: i32 = a.iter().inspect(|&&x| seen.push(x)).filter(|&&x| x % 3 == 0).sum();
    println!("inspect: sum {} saw {}", s, seen.len());
    println!("rev/last/nth: {:?} {:?} {:?} {:?}", a.iter().rev().next(), a.iter().last(), a.iter().nth(3), a.iter().nth(20));
    let mut it = a.iter();
    println!("nth_back: {:?} {:?} then next_back {:?}", it.nth_back(2), it.nth(0), it.next_back());
    println!("min_by_key: {:?} max_by_key: {:?}", words.iter().min_by_key(|w| w.len()), words.iter().max_by_key(|w| w.len()));
    println!("max_by: {:?} min_by: {:?}", words.iter().max_by(|x, y| x.to_lowercase().cmp(&y.to_lowercase())), words.iter().min_by(|x, y| x.len().cmp(&y.len()).then(y.cmp(x))));
    println!("max/min ties: {:?} {:?}", [(1, 'a'), (3, 'b'), (3, 'c'), (1, 'd')].iter().max_by_key(|p| p.0), [(1, 'a'), (3, 'b'), (3, 'c'), (1, 'd')].iter().min_by_key(|p| p.0));
    println!("position {:?} rposition {:?} find {:?} find_map {:?}", a.iter().position(|&x| x > 4), a.iter().rposition(|&x| x < 4), words.iter().find(|w| w.starts_with('D')), words.iter().find_map(|w| w.strip_prefix('e')));
    println!("sum {} product {} float sum {}", a.iter().sum::<i32>(), a.iter().take(6).product::<i32>(), [0.1, 0.2, 0.3].iter().sum::<f64>());
    let opt_sum: Option<i32> = [Some(1), Some(2)].into_iter().sum();
    let opt_none: Option<i32> = [Some(1), None].into_iter().sum();
    let res_prod: Result<i32, &str> = [Ok(2), Ok(3), Err("bad"), Err("worse")].into_iter().product();
    println!("sum Option {:?} {:?} product Result {:?}", opt_sum, opt_none, res_prod);
    let collected: Result<Vec<i32>, std::num::ParseIntError> = "1 2 3".split(' ').map(str::parse).collect();
    let failed: Result<Vec<i32>, String> = "1 x 3".split(' ').map(|s| s.parse().map_err(|_| format!("bad {:?}", s))).collect();
    let opts: Option<Vec<char>> = ["a", "b"].iter().map(|s| s.chars().next()).collect();
    println!("collect Result {:?} {:?} Option {:?}", collected, failed, opts);
    println!("cmp {:?} {:?} {:?}", [1, 2, 3].iter().cmp([1, 2].iter()), [1, 2].iter().cmp([1, 3].iter()), iter::empty::<i32>().cmp(iter::empty()));
    println!("eq {} ne {} lt {} le {} gt {} ge {}", (1..4).eq([1, 2, 3]), (1..4).ne(1..5), (1..3).lt(1..4), [2].iter().le([1, 9].iter()), "b".chars().gt("abc".chars()), (0..0).ge(0..0));
    println!("partial_cmp floats {:?} {:?}", [1.0, f64::NAN].iter().partial_cmp([1.0, 2.0].iter()), [1.0, 2.0].iter().partial_cmp([1.0, 3.0].iter()));
    println!("is_sorted {} {}", a.iter().is_sorted(), words.iter().is_sorted_by_key(|w| w.len()));
    let mut pk = a.iter().peekable();
    let mut groups = vec![];
    while let Some(&x) = pk.next() {
        let mut g = vec![x];
        while let Some(y) = pk.next_if(|&&y| y / 4 == x / 4) {
            g.push(*y);
        }
        groups.push(g);
    }
    println!("peekable next_if groups: {:?}", groups);
    let mut pk2 = [1, 1, 2].iter().peekable();
    println!("next_if_eq {:?} {:?} peek {:?}", pk2.next_if_eq(&&1), pk2.next_if_eq(&&2), pk2.peek());
    println!("chain/cycle: {:?}", (1..3).chain(7..9).cycle().take(9).collect::<Vec<_>>());
    println!("successors: {:?}", iter::successors(Some(1u64), |&x| if x < 1000 { Some(x * 3) } else { None }).collect::<Vec<_>>());
    let mut st = (0, 1);
    println!("from_fn fib: {:?}", iter::from_fn(|| { let r = st.0; st = (st.1, st.0 + st.1); Some(r) }).take(12).collect::<Vec<_>>());
    let mut c = 0;
    println!("repeat_with: {:?}", iter::repeat_with(|| { c += 2; c }).take(4).collect::<Vec<_>>());
    println!("repeat_n: {:?} once: {:?}", iter::repeat_n("ab", 3).collect::<String>(), iter::once(5).chain(iter::once(6)).collect::<Vec<_>>());
    println!("enumerate rev: {:?}", words.iter().enumerate().rev().take(2).collect::<Vec<_>>());
    println!("zip rev: {:?}", a.iter().zip(words.iter()).rev().take(2).collect::<Vec<_>>());
    let ex = a.iter().skip(2).step_by(3);
    println!("ExactSize len {} size_hint {:?}", ex.len(), (0..10).filter(|x| x % 2 == 0).size_hint());
    println!("any {} all {} count {}", a.iter().any(|&x| x > 9), a.iter().all(|&x| x > 0), words.iter().filter(|w| w.len() == 3).count());
    let mut by = a.iter();
    let first3: Vec<_> = by.by_ref().take(3).collect();
    let rest: i32 = by.sum();
    println!("by_ref: {:?} rest sum {}", first3, rest);
    let mut fused = [Some(1), None, Some(3)].into_iter().map_while(|x| x).fuse();
    println!("fuse: {:?} {:?} {:?}", fused.next(), fused.next(), fused.next());
    println!("map rev: {:?}", (1..=4).map(|x| x * x).rev().collect::<Vec<_>>());
    println!("sum of squares of odd: {}", (1..=99).filter(|n| n % 2 == 1).map(|n| n * n).sum::<u64>());
    println!("min/max empty: {:?} {:?}", iter::empty::<u8>().max(), Vec::<f64>::new().iter().cloned().reduce(f64::min));
    println!("reduce: {:?}", words.iter().map(|w| w.to_string()).reduce(|a, b| a + "," + &b));
    println!("collect String from chars rev: {}", "stressed".chars().rev().collect::<String>());
    println!("unzip3-ish: {:?}", [(1, 'a', "x"), (2, 'b', "y")].iter().fold((vec![], String::new(), vec![]), |(mut n, mut c, mut s), &(x, y, w)| { n.push(x); c.push(y); s.push(w); (n, c, s) }));
    println!("array map / zip: {:?} {:?}", [1, 2, 3].map(|x| x * 10), [1, 2].iter().zip([3, 4].iter()).map(|(x, y)| x * y).collect::<Vec<_>>());
    let windows_max: Vec<i32> = a.windows(3).map(|w| *w.iter().max().unwrap()).collect();
    println!("windows max: {:?}", windows_max);
    println!("last of map with side effects: {:?}", (1..=5).map(|x| x * 2).last());
    println!("double ended chain: {:?}", (1..3).chain(5..7).rev().collect::<Vec<_>>());
    println!("rev chars of graphemes-ish: {:?}", "héllo".char_indices().rev().map(|(i, c)| format!("{}{}", i, c)).collect::<Vec<_>>());
    println!("array_chunks via chunks_exact: {:?}", a.chunks_exact(4).map(|c| c.iter().product::<i32>()).collect::<Vec<_>>());
    println!("intersperse-like: {}", words.iter().take(3).copied().collect::<Vec<_>>().join(" | "));
    println!("max_by partial floats: {:?}", [2.5, -1.0, 9.75, 3.0].iter().cloned().max_by(|a: &f64, b| a.total_cmp(b)));
}
