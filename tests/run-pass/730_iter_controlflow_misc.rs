// Less common iterator APIs: try_for_each/try_fold with ControlFlow,
// next_if/next_if_eq, by_ref, cmp/partial_cmp/eq/lt on iterators,
// max_by_key/min_by_key tie behavior, once_with, cycle, inspect, flatten of
// options, array IntoIter, rev + skip_while, find_map, reduce, sum of floats.
use std::ops::ControlFlow;

#[derive(Debug, Clone, Copy, PartialEq)]
struct Item { id: u32, score: i32 }

fn main() {
    let v = [3, 8, 1, 9, 4, 10, 2];
    let r = v.iter().try_for_each(|&x| if x > 8 { ControlFlow::Break(x) } else { ControlFlow::Continue(()) });
    println!("try_for_each: {:?}", r);
    let r2: ControlFlow<i32, i32> = v.iter().try_fold(0, |acc, &x| {
        let n = acc + x;
        if n > 20 { ControlFlow::Break(n) } else { ControlFlow::Continue(n) }
    });
    println!("try_fold controlflow: {:?} is_break={}", r2, r2.is_break());
    let checked: Option<u8> = [100u8, 100, 50, 10].iter().try_fold(0u8, |a, &b| a.checked_add(b));
    println!("checked try_fold: {:?}", checked);
    let res: Result<i32, String> = ["1", "2", "x3", "4"].iter().try_fold(0, |a, s| s.parse::<i32>().map(|n| a + n).map_err(|e| format!("{}: {}", s, e)));
    println!("result try_fold: {:?}", res);

    let mut it = [1, 1, 2, 3, 3, 3, 4].into_iter().peekable();
    let mut groups = Vec::new();
    while let Some(x) = it.next() {
        let mut n = 1;
        while it.next_if_eq(&x).is_some() { n += 1; }
        groups.push((x, n));
    }
    println!("next_if_eq groups: {:?}", groups);
    let mut digits = "123abc45".chars().peekable();
    let mut num = String::new();
    while let Some(c) = digits.next_if(|c| c.is_ascii_digit()) { num.push(c); }
    println!("next_if digits: {} rest: {}", num, digits.collect::<String>());

    let mut src = (1..=10).into_iter();
    let first3: Vec<_> = src.by_ref().take(3).collect();
    let evens: Vec<_> = src.by_ref().filter(|x| x % 2 == 0).take(2).collect();
    let rest: Vec<_> = src.collect();
    println!("by_ref: {:?} {:?} {:?}", first3, evens, rest);

    println!("cmp: {:?} {:?} {:?}", [1, 2, 3].iter().cmp([1, 2].iter()), [1, 2].iter().cmp([1, 3].iter()), (0..0).cmp(0..0));
    println!("partial_cmp: {:?} {:?}", [1.0, f64::NAN].iter().partial_cmp([1.0, 2.0].iter()), [1.0].iter().partial_cmp([0.5, 9.0].iter()));
    println!("eq/lt/ge: {} {} {} {}", (1..4).eq([1, 2, 3]), "abc".chars().lt("abd".chars()), [2].iter().ge([1, 9].iter()), "".chars().le("a".chars()));

    let items = [Item { id: 1, score: 5 }, Item { id: 2, score: 9 }, Item { id: 3, score: 9 }, Item { id: 4, score: 1 }, Item { id: 5, score: 1 }];
    println!("max_by_key (last of ties): {:?}", items.iter().max_by_key(|i| i.score).map(|i| i.id));
    println!("min_by_key (first of ties): {:?}", items.iter().min_by_key(|i| i.score).map(|i| i.id));
    println!("max_by rev: {:?}", items.iter().rev().max_by(|a, b| a.score.cmp(&b.score)).map(|i| i.id));
    println!("min (tuple): {:?}", items.iter().map(|i| (i.score, std::cmp::Reverse(i.id))).min());

    let once: Vec<String> = std::iter::once_with(|| "lazy".to_string()).chain(std::iter::once("eager".to_string())).collect();
    println!("once_with: {:?}", once);
    let cyc: String = "abc".chars().cycle().skip(2).take(7).collect();
    println!("cycle: {}", cyc);
    let mut seen = Vec::new();
    let s: i32 = (1..=5).inspect(|x| seen.push(*x)).filter(|x| x % 2 == 1).map(|x| x * x).sum();
    println!("inspect saw {:?} sum {}", seen, s);
    let nested = vec![Some(vec![1, 2]), None, Some(vec![]), Some(vec![3])];
    let flat: Vec<i32> = nested.into_iter().flatten().flatten().collect();
    println!("flatten twice: {:?}", flat);
    let arr = [String::from("x"), String::from("yy"), String::from("zzz")];
    let moved: Vec<String> = arr.into_iter().rev().collect();
    println!("array into_iter rev: {:?}", moved);
    let trailing: Vec<i32> = [1, 0, 3, 0, 0].iter().rev().skip_while(|&&x| x == 0).copied().collect::<Vec<_>>().into_iter().rev().collect();
    println!("strip trailing zeros: {:?}", trailing);
    println!("find_map: {:?}", ["a", "12", "b", "7"].iter().find_map(|s| s.parse::<u8>().ok()));
    println!("reduce: {:?} {:?}", (1..=5).reduce(|a, b| a * b), (0..0).reduce(|a: i32, b| a + b));
    let fs = [0.1f32, 0.2, 0.3, 0.4];
    println!("float sum f32={} f64={} product={}", fs.iter().sum::<f32>(), fs.iter().map(|&x| x as f64).sum::<f64>(), fs.iter().product::<f32>());
    println!("position/rposition: {:?} {:?}", v.iter().position(|&x| x > 5), v.iter().rposition(|&x| x > 5));
    println!("last/nth: {:?} {:?} {:?}", v.iter().last(), v.iter().nth(3), v.iter().nth(30));
    println!("skip+step: {:?}", (0..20).skip(3).step_by(4).collect::<Vec<_>>());
    println!("count chars: {} any {} all {}", "hello".chars().filter(|c| "aeiou".contains(*c)).count(), v.iter().any(|&x| x == 9), v.iter().all(|&x| x > 0));
    let (mut lo, mut hi) = (vec![], vec![]);
    v.iter().for_each(|&x| if x < 5 { lo.push(x) } else { hi.push(x) });
    println!("for_each split: {:?} {:?}", lo, hi);
    let sums: Vec<i32> = v.windows(3).map(|w| w.iter().sum()).collect();
    println!("window sums: {:?}, chunks_exact rem: {:?}", sums, v.chunks_exact(3).remainder());
    println!("rchunks: {:?}", v.rchunks(3).collect::<Vec<_>>());
    println!("split on >8: {:?}", v.split(|&x| x > 8).collect::<Vec<_>>());
    println!("splitn: {:?}", v.splitn(2, |&x| x == 1).collect::<Vec<_>>());
    println!("rsplit_inclusive? {:?}", v.split_inclusive(|&x| x % 2 == 0).collect::<Vec<_>>());
}
