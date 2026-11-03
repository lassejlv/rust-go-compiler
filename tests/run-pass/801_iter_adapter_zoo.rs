fn main() {
    let data = vec![3, 1, 4, 1, 5, 9, 2, 6, 5, 3, 5, 8, 9, 7, 9, 3, 2, 3, 8, 4];

    // peekable run-length grouping
    let mut it = data.iter().copied().peekable();
    let mut runs = Vec::new();
    while let Some(x) = it.next() {
        let mut n = 1;
        while it.peek().map_or(false, |&y| (y > x) == (x % 2 == 0)) {
            it.next();
            n += 1;
        }
        runs.push((x, n));
    }
    println!("runs: {:?}", runs);

    // scan prefix sums with early stop
    let prefix: Vec<i32> = data.iter().scan(0, |acc, &x| { *acc += x; if *acc > 50 { None } else { Some(*acc) } }).collect();
    println!("prefix<=50: {:?}", prefix);

    // take_while / skip_while
    let tw: Vec<_> = data.iter().take_while(|&&x| x < 9).collect();
    let sw: Vec<_> = data.iter().skip_while(|&&x| x != 9).skip(1).collect();
    println!("take_while: {:?}\nskip_while: {:?}", tw, sw);

    // step_by on ranges and iter
    let sb: Vec<_> = (0..30).step_by(7).collect();
    let sb2: Vec<_> = data.iter().step_by(3).rev().collect();
    println!("step_by: {:?} {:?}", sb, sb2);

    // chunks / windows / chunks_exact / rchunks
    for c in data.chunks(6) { print!("{:?};", c.iter().sum::<i32>()); }
    println!();
    let incr = data.windows(3).filter(|w| w[0] < w[1] && w[1] < w[2]).count();
    println!("increasing triples: {}", incr);
    let ce = data.chunks_exact(7);
    println!("remainder: {:?}", ce.remainder());
    let rc: Vec<Vec<i32>> = data.rchunks(6).map(|c| c.to_vec()).collect();
    println!("rchunks: {:?}", rc);

    // zip / unzip
    let (evens, odds): (Vec<usize>, Vec<i32>) = data.iter().enumerate().filter(|(i, _)| i % 4 == 0).map(|(i, &v)| (i, v * 10)).unzip();
    println!("unzip: {:?} {:?}", evens, odds);
    let pairs: Vec<(char, i32)> = "abcdef".chars().zip(data.iter().rev().copied()).collect();
    println!("zip: {:?}", pairs);

    // flat_map + fuse
    let fm: Vec<String> = (1..4).flat_map(|n| (0..n).map(move |k| format!("{}{}", n, k))).collect();
    println!("flat_map: {:?}", fm);
    struct Flaky { n: u32 }
    impl Iterator for Flaky {
        type Item = u32;
        fn next(&mut self) -> Option<u32> {
            self.n += 1;
            if self.n % 3 == 0 { None } else { Some(self.n) }
        }
    }
    let mut f = Flaky { n: 0 }.fuse();
    let got: Vec<Option<u32>> = (0..5).map(|_| f.next()).collect();
    println!("fused: {:?}", got);
    let mut raw = Flaky { n: 0 };
    let got2: Vec<Option<u32>> = (0..5).map(|_| raw.next()).collect();
    println!("raw:   {:?}", got2);

    // rev on chain
    let ch: Vec<i32> = (1..4).chain(10..13).rev().collect();
    println!("rev chain: {:?}", ch);
    let ch2: Vec<i32> = data[..3].iter().chain(data[17..].iter()).rev().map(|x| x * x).collect();
    println!("rev chain2: {:?}", ch2);

    // position / rposition / find_map / max_by_key / min_by
    println!("pos9={:?} rpos9={:?}", data.iter().position(|&x| x == 9), data.iter().rposition(|&x| x == 9));
    let fmz = data.iter().find_map(|&x| if x > 6 { Some(x * 100) } else { None });
    println!("find_map: {:?}", fmz);
    let mbk = data.iter().enumerate().max_by_key(|&(_, v)| *v);
    let mnb = data.iter().enumerate().min_by(|a, b| a.1.cmp(b.1).then(b.0.cmp(&a.0)));
    println!("max_by_key: {:?} min_by: {:?}", mbk, mnb);

    // cycle + take + skip, last, nth, count
    let cyc: String = "xyz".chars().cycle().skip(2).take(7).collect();
    println!("cycle: {}", cyc);
    let mut it2 = data.iter();
    println!("nth(2)={:?} nth(0)={:?} last={:?}", it2.nth(2), it2.nth(0), it2.clone().last());
    println!("remaining count = {}", it2.count());

    // partition, any, all, sum, product
    let (big, small): (Vec<i32>, Vec<i32>) = data.iter().partition(|&&x| x >= 5);
    println!("big={:?} small={:?}", big, small);
    println!("any>8 {} all>0 {}", data.iter().any(|&x| x > 8), data.iter().all(|&x| x > 0));
    let prod: i64 = data.iter().take(8).map(|&x| x as i64).product();
    println!("product: {}", prod);

    // inspect & map_while
    let mut seen = Vec::new();
    let mw: Vec<i32> = data.iter().inspect(|x| seen.push(**x)).map_while(|&x| if x != 2 { Some(x * 2) } else { None }).collect();
    println!("map_while: {:?} seen={:?}", mw, seen);

    // try_fold
    let tf: Result<i32, String> = data.iter().try_fold(0i32, |acc, &x| if acc + x > 40 { Err(format!("overflow at {}", x)) } else { Ok(acc + x) });
    println!("try_fold: {:?}", tf);
    let tf2: Option<i32> = data.iter().try_fold(0i32, |acc, &x| acc.checked_add(x));
    println!("try_fold2: {:?}", tf2);
}
