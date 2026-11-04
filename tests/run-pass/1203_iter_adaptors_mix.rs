fn main() {
    let v: Vec<i32> = (1..=12).collect();

    // peekable grouping
    let mut it = v.iter().peekable();
    let mut groups = Vec::new();
    while let Some(&x) = it.next() {
        let mut g = vec![x];
        while let Some(&&y) = it.peek() {
            if y / 4 == x / 4 {
                g.push(y);
                it.next();
            } else {
                break;
            }
        }
        groups.push(g);
    }
    println!("{:?}", groups);

    // next_if / next_if_eq
    let mut p = "aaabccdd".chars().peekable();
    let mut runs = vec![];
    while let Some(c) = p.next() {
        let mut n = 1;
        while p.next_if_eq(&c).is_some() {
            n += 1;
        }
        runs.push(format!("{}{}", c, n));
    }
    println!("{}", runs.join(","));
    let mut digits = "123abc".chars().peekable();
    let mut num = 0;
    while let Some(d) = digits.next_if(|c| c.is_ascii_digit()) {
        num = num * 10 + d.to_digit(10).unwrap();
    }
    println!("{} rest={}", num, digits.collect::<String>());

    // scan with early stop
    let running: Vec<i32> = v
        .iter()
        .scan(0, |acc, &x| {
            *acc += x;
            if *acc > 40 { None } else { Some(*acc) }
        })
        .collect();
    println!("{:?}", running);

    // flat_map + rev + zip + step_by
    let fm: Vec<(i32, char)> = (1..4)
        .flat_map(|i| "ab".chars().map(move |c| (i, c)))
        .rev()
        .collect();
    println!("{:?}", fm);
    let z: Vec<(i32, i32)> = v.iter().copied().step_by(3).zip(v.iter().rev().copied().step_by(4)).collect();
    println!("{:?}", z);
    let sb: Vec<i32> = (0..20).rev().step_by(6).collect();
    println!("{:?}", sb);
    let sb2: Vec<i32> = (0..20).step_by(6).rev().collect();
    println!("{:?}", sb2);

    // windows / chunks / chunks_exact / rchunks
    let w: Vec<i32> = v.windows(3).map(|w| w.iter().product()).take(4).collect();
    println!("{:?}", w);
    let ce = v.chunks_exact(5);
    println!("rem={:?}", ce.remainder());
    let sums: Vec<i32> = ce.map(|c| c.iter().sum()).collect();
    println!("{:?}", sums);
    let rc: Vec<Vec<i32>> = v.rchunks(5).map(|c| c.to_vec()).collect();
    println!("{:?}", rc);
    let ch: Vec<usize> = v.chunks(5).map(|c| c.len()).rev().collect();
    println!("{:?}", ch);

    // rsplitn, splitn, rsplit, split_terminator
    let path = "a/b/c/d.txt";
    let parts: Vec<&str> = path.rsplitn(2, '/').collect();
    println!("{:?}", parts);
    let parts: Vec<&str> = path.splitn(3, '/').collect();
    println!("{:?}", parts);
    let parts: Vec<&str> = path.rsplit('/').collect();
    println!("{:?}", parts);
    let parts: Vec<&str> = "x;y;z;".split_terminator(';').collect();
    println!("{:?}", parts);
    let parts: Vec<&str> = "  lots   of   space ".split_whitespace().rev().collect();
    println!("{:?}", parts);
    let sl: Vec<Vec<i32>> = v.split(|x| x % 5 == 0).map(|s| s.to_vec()).collect();
    println!("{:?}", sl);
    let sl: Vec<Vec<i32>> = v.rsplitn(2, |x| x % 4 == 0).map(|s| s.to_vec()).collect();
    println!("{:?}", sl);

    // take_while, skip_while, map_while, inspect, fuse, cycle
    let tw: Vec<i32> = v.iter().copied().skip_while(|&x| x < 4).take_while(|&x| x < 9).collect();
    println!("{:?}", tw);
    let mw: Vec<u32> = ["1", "2", "x", "4"].iter().map_while(|s| s.parse().ok()).collect();
    println!("{:?}", mw);
    let mut seen = vec![];
    let total: i32 = v.iter().inspect(|&&x| if x % 5 == 0 { seen.push(x) }).sum();
    println!("{} {:?}", total, seen);
    let cy: String = "abc".chars().cycle().skip(2).take(7).collect();
    println!("{}", cy);

    // position / rposition / find_map / max_by_key / min_by / last / nth / nth_back
    println!("{:?} {:?}", v.iter().position(|&x| x > 5), v.iter().rposition(|&x| x < 5));
    println!("{:?}", ["a", "12", "b", "7"].iter().find_map(|s| s.parse::<i32>().ok()));
    let words = ["pear", "fig", "banana", "kiwi", "plum"];
    println!("{:?} {:?}", words.iter().max_by_key(|w| w.len()), words.iter().min_by_key(|w| w.len()));
    println!("{:?}", words.iter().max_by(|a, b| a.len().cmp(&b.len()).then(b.cmp(a))));
    let mut it = v.iter();
    println!("{:?} {:?} {:?}", it.nth(2), it.nth_back(1), it.len());
    println!("{:?}", words.iter().rev().skip(1).last());

    // partition, unzip, chain, enumerate rev
    let (even, odd): (Vec<i32>, Vec<i32>) = v.iter().partition(|&&x| x % 2 == 0);
    println!("{:?} {:?}", even, odd);
    let (a, b): (Vec<usize>, Vec<char>) = "xyz".chars().enumerate().unzip();
    println!("{:?} {:?}", a, b);
    let e: Vec<(usize, &i32)> = v.iter().enumerate().rev().take(2).collect();
    println!("{:?}", e);
    let ch: Vec<i32> = (1..3).chain(7..9).chain(std::iter::once(100)).collect();
    println!("{:?}", ch);

    // successors, from_fn, repeat_with
    let pow: Vec<u64> = std::iter::successors(Some(1u64), |&x| if x < 500 { Some(x * 3) } else { None }).collect();
    println!("{:?}", pow);
    let mut st = (0u32, 1u32);
    let fib: Vec<u32> = std::iter::from_fn(|| {
        let r = st.0;
        st = (st.1, st.0 + st.1);
        Some(r)
    })
    .take(10)
    .collect();
    println!("{:?}", fib);
    let mut k = 1;
    let rw: Vec<i32> = std::iter::repeat_with(|| { k *= 2; k }).take(5).collect();
    println!("{:?}", rw);

    // try_fold, all, any, count, sum of floats, product overflow-safe
    let r: Option<i32> = v.iter().try_fold(0i32, |acc, &x| acc.checked_add(x * 1000000));
    println!("{:?}", r);
    let r: Result<i32, String> = v.iter().try_fold(0, |acc, &x| if x == 7 { Err(format!("hit {}", x)) } else { Ok(acc + x) });
    println!("{:?}", r);
    println!("{} {} {}", v.iter().all(|x| *x > 0), v.iter().any(|x| *x > 11), v.iter().filter(|x| **x % 3 == 0).count());
    let fsum: f64 = [0.1, 0.2, 0.3].iter().sum();
    println!("{}", fsum);
    let cmp = [1, 2, 3].iter().cmp([1, 2, 4].iter());
    println!("{:?} {}", cmp, [1.0, 2.0].iter().partial_cmp([1.0].iter()) == Some(std::cmp::Ordering::Greater));
    println!("{}", (1..=4).map(|x| x.to_string()).collect::<Vec<_>>().join("+"));
    let s: String = vec!["ab", "cd"].concat();
    println!("{}", s);
    let rev_sorted = {
        let mut x = words.to_vec();
        x.sort_by(|a, b| b.cmp(a));
        x
    };
    println!("{:?}", rev_sorted);
    println!("{:?}", (1..=5).reduce(|a, b| a * b));
    println!("{}", v.iter().rev().map(|x| x.to_string()).collect::<String>());
    println!("{:?}", "hello".char_indices().filter(|(_, c)| *c == 'l').map(|(i, _)| i).collect::<Vec<_>>());
    println!("{:?}", v.iter().min_by(|a, b| (*a % 5).cmp(&(*b % 5))));
    println!("{}", v.iter().is_sorted());
}
