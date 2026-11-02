// try_fold / try_for_each with Result and Option and ControlFlow, rposition, rfind, rfold,
// reduce, max/min with ties, is_sorted, sum of Results, and short-circuiting checks.
use std::ops::ControlFlow;

fn parse_all(items: &[&str]) -> Result<i32, String> {
    items.iter().try_fold(0i32, |acc, s| {
        let v: i32 = s.trim().parse().map_err(|e| format!("bad {:?}: {}", s, e))?;
        acc.checked_add(v).ok_or_else(|| "overflow".to_string())
    })
}

fn first_over(v: &[u32], limit: u32) -> ControlFlow<(usize, u32), u32> {
    v.iter().enumerate().try_fold(0u32, |sum, (i, &x)| {
        let s = sum + x;
        if s > limit {
            ControlFlow::Break((i, s))
        } else {
            ControlFlow::Continue(s)
        }
    })
}

fn main() {
    println!("{:?}", parse_all(&["1", " 2", "3 "]));
    println!("{:?}", parse_all(&["1", "x", "3"]));
    println!("{:?}", parse_all(&["2147483647", "1"]));
    println!("{:?}", parse_all(&[]));

    println!("{:?}", first_over(&[5, 10, 20, 40], 30));
    println!("{:?}", first_over(&[1, 2, 3], 30));

    let opt_sum: Option<u8> = [100u8, 100, 50].iter().try_fold(0u8, |a, &b| a.checked_add(b));
    let opt_sum2: Option<u8> = [100u8, 100, 60].iter().try_fold(0u8, |a, &b| a.checked_add(b));
    println!("{:?} {:?}", opt_sum, opt_sum2);

    let mut visited = Vec::new();
    let r: Result<(), i32> = (1..10).try_for_each(|x| {
        visited.push(x);
        if x * x > 20 { Err(x) } else { Ok(()) }
    });
    println!("{:?} visited {:?}", r, visited);

    let v = [3, 8, 1, 8, 5, 8, 2];
    println!("position {:?} rposition {:?}", v.iter().position(|&x| x == 8), v.iter().rposition(|&x| x == 8));
    println!("rfind {:?}", v.iter().rfind(|&&x| x < 5));
    let rf = v.iter().rfold(String::new(), |mut s, x| {
        s.push_str(&x.to_string());
        s
    });
    println!("rfold {}", rf);
    println!("reduce {:?}", v.iter().copied().reduce(|a, b| a * 10 + b));
    println!("reduce empty {:?}", Vec::<i32>::new().into_iter().reduce(|a, b| a + b));

    // max returns last max, min returns first min
    let pairs = [(1, 'a'), (3, 'b'), (2, 'c'), (3, 'd'), (1, 'e')];
    println!("max_by_key {:?}", pairs.iter().max_by_key(|p| p.0));
    println!("min_by_key {:?}", pairs.iter().min_by_key(|p| p.0));
    println!("rev max_by_key {:?}", pairs.iter().rev().max_by_key(|p| p.0));

    println!("sorted {} {}", [1, 2, 2, 5].is_sorted(), [3, 1].is_sorted());
    println!("sorted_by_key {}", ["a", "bb", "ccc"].is_sorted_by_key(|s| s.len()));

    let results = vec![Ok(1), Ok(2), Err("e1"), Ok(4), Err("e2")];
    let s: Result<i32, &str> = results.iter().cloned().sum();
    let s2: Result<i32, &str> = results.iter().cloned().take(2).sum();
    let p: Option<i32> = [Some(2), Some(3), Some(4)].into_iter().product();
    println!("{:?} {:?} {:?}", s, s2, p);

    let collected: Result<Vec<i32>, &str> = results.iter().cloned().collect();
    let oks: Vec<i32> = results.iter().filter_map(|r| r.ok()).collect();
    let errs: Vec<&str> = results.iter().filter_map(|r| r.err()).collect();
    println!("{:?} {:?} {:?}", collected, oks, errs);

    let mut calls = 0;
    let found = (1..).map(|x| {
        calls += 1;
        x * 3
    }).find(|x| x % 7 == 0);
    println!("found {:?} after {} calls", found, calls);

    let cf = [1, 2, 3, 4].iter().try_for_each(|&x| {
        if x == 3 { ControlFlow::Break(x * 100) } else { ControlFlow::Continue(()) }
    });
    println!("{:?} is_break={}", cf, cf.is_break());

    let nested: Result<Vec<Vec<u8>>, std::num::ParseIntError> = ["1,2", "3", "4,5,6"]
        .iter()
        .map(|line| line.split(',').map(|n| n.parse::<u8>()).collect())
        .collect();
    println!("{:?}", nested);
    let bad: Result<Vec<Vec<u8>>, std::num::ParseIntError> = ["1,2", "300"]
        .iter()
        .map(|line| line.split(',').map(|n| n.parse::<u8>()).collect())
        .collect();
    println!("{:?}", bad.map_err(|e| e.to_string()));
}
