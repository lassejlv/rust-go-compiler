// Breadth test of Vec and slice APIs.
fn main() {
    // retain / retain_mut / extract_if / dedup family
    let mut v: Vec<i32> = (1..=20).collect();
    v.retain(|x| x % 3 != 0);
    println!("retain: {:?}", v);
    v.retain_mut(|x| {
        *x *= 10;
        *x % 40 != 0
    });
    println!("retain_mut: {:?}", v);
    let mut w: Vec<i32> = (1..=12).collect();
    let evens: Vec<i32> = w.extract_if(.., |x| *x % 2 == 0).collect();
    println!("extract_if: taken {:?} left {:?}", evens, w);
    let mut w2: Vec<i32> = (0..10).collect();
    let mid: Vec<i32> = w2.extract_if(2..8, |x| *x % 3 == 0).collect();
    println!("extract_if range: {:?} {:?}", mid, w2);
    let mut d = vec![1, 1, 2, 3, 3, 3, 1, 4, 4];
    d.dedup();
    println!("dedup: {:?}", d);
    let mut d2 = vec![10, 11, 20, 25, 31, 39, 40];
    d2.dedup_by_key(|x| *x / 10);
    println!("dedup_by_key: {:?}", d2);
    let mut d3 = vec!["a", "A", "b", "B", "b", "c"];
    d3.dedup_by(|a, b| a.eq_ignore_ascii_case(b));
    println!("dedup_by: {:?}", d3);

    // rotate / swap / reverse / fill / copy_within
    let mut r: Vec<char> = "abcdefg".chars().collect();
    r.rotate_left(2);
    println!("rotate_left: {}", r.iter().collect::<String>());
    r.rotate_right(3);
    println!("rotate_right: {}", r.iter().collect::<String>());
    r.swap(0, 6);
    r[1..5].reverse();
    println!("swap+reverse: {}", r.iter().collect::<String>());
    let mut f = [0u8; 8];
    f[2..6].fill(7);
    f.copy_within(2..4, 5);
    println!("fill/copy_within: {:?}", f);
    let mut g = vec![1, 2, 3];
    g.fill_with(Default::default);
    println!("fill_with: {:?}", g);

    // windows / chunks / rchunks / exact variants
    let s = [1, 2, 3, 4, 5, 6, 7];
    println!("windows: {:?}", s.windows(3).map(|w| w.iter().sum::<i32>()).collect::<Vec<_>>());
    println!("chunks: {:?}", s.chunks(3).collect::<Vec<_>>());
    println!("rchunks: {:?}", s.rchunks(3).collect::<Vec<_>>());
    let ce = s.chunks_exact(2);
    println!("chunks_exact: {:?} rem {:?}", ce.clone().collect::<Vec<_>>(), ce.remainder());
    let rce = s.rchunks_exact(3);
    println!("rchunks_exact: {:?} rem {:?}", rce.clone().collect::<Vec<_>>(), rce.remainder());
    println!("chunk_by: {:?}", [1, 1, 2, 3, 3, 3, 2, 2].chunk_by(|a, b| a == b).collect::<Vec<_>>());
    println!("chunk_by asc: {:?}", [1, 2, 3, 2, 3, 4, 1].chunk_by(|a, b| a < b).map(|c| c.len()).collect::<Vec<_>>());
    let mut cm = [1, 2, 3, 4, 5];
    for c in cm.chunks_mut(2) {
        c.reverse();
    }
    println!("chunks_mut: {:?}", cm);

    // split family
    let data = [1, 0, 2, 3, 0, 0, 4];
    println!("split: {:?}", data.split(|x| *x == 0).collect::<Vec<_>>());
    println!("splitn: {:?}", data.splitn(2, |x| *x == 0).collect::<Vec<_>>());
    println!("rsplit: {:?}", data.rsplit(|x| *x == 0).collect::<Vec<_>>());
    println!("split_inclusive: {:?}", data.split_inclusive(|x| *x == 0).collect::<Vec<_>>());
    let (a, b) = data.split_at(3);
    println!("split_at: {:?} {:?}", a, b);
    println!("split_first: {:?} split_last: {:?}", data.split_first(), data.split_last());
    println!("first/last: {:?} {:?} {:?}", data.first(), data.last(), Vec::<i32>::new().first());

    // select_nth_unstable / partition_point / binary search
    let mut sel = vec![9, 1, 8, 2, 7, 3, 6, 4, 5];
    let (_, median, _) = sel.select_nth_unstable(4);
    println!("median: {}", median);
    let mut parts = sel.clone();
    parts.select_nth_unstable_by(2, |a, b| b.cmp(a));
    let mut top3 = parts[..3].to_vec();
    top3.sort();
    println!("top3: {:?}", top3);
    let sorted = [1, 2, 4, 4, 4, 7, 9];
    println!("partition_point <4: {} <=4: {}", sorted.partition_point(|&x| x < 4), sorted.partition_point(|&x| x <= 4));
    println!("binary_search 7: {:?} 5: {:?} 0: {:?} 10: {:?}", sorted.binary_search(&7), sorted.binary_search(&5), sorted.binary_search(&0), sorted.binary_search(&10));
    let people = [("ann", 21), ("bob", 30), ("cat", 30), ("dan", 45)];
    println!("binary_search_by_key 45: {:?} 22: {:?}", people.binary_search_by_key(&45, |p| p.1), people.binary_search_by_key(&22, |p| p.1));
    println!("binary_search_by: {:?}", people.binary_search_by(|p| p.0.cmp("cat")));

    // sorting
    let mut words = vec!["delta", "Alpha", "charlie", "bravo", "Echo", "alpha"];
    words.sort();
    println!("sort: {:?}", words);
    words.sort_unstable_by_key(|w| w.to_lowercase());
    println!("sort_unstable_by_key (lower): {:?}", words.iter().map(|w| w.to_lowercase()).collect::<Vec<_>>());
    words.sort_by_key(|w| std::cmp::Reverse(w.len()));
    println!("sort_by_key len desc (stable): {:?}", words);
    words.sort_by_cached_key(|w| w.chars().rev().collect::<String>());
    println!("sort_by_cached_key rev: {:?}", words);
    let mut fl = vec![3.5, -0.0, 0.0, f64::INFINITY, -1.25, f64::NEG_INFINITY];
    fl.sort_by(|a, b| a.total_cmp(b));
    println!("floats total_cmp: {:?}", fl);
    println!("is_sorted: {} {} {}", [1, 2, 2, 5].is_sorted(), [3, 1].is_sorted(), words.is_sorted_by_key(|w| w.len()));

    // concat / join / repeat
    let nested = vec![vec![1, 2], vec![], vec![3]];
    println!("concat: {:?}", nested.concat());
    println!("join: {:?}", [[1, 2], [3, 4]].join(&0));
    println!("join str: {}", ["a", "b", "c"].join("->"));
    println!("concat str: {}", ["x", "y", "z"].concat());
    println!("repeat: {:?}", [1, 2].repeat(3));

    // split_off / append / drain / splice / truncate / resize
    let mut base: Vec<i32> = (0..10).collect();
    let tail = base.split_off(6);
    println!("split_off: {:?} {:?}", base, tail);
    let mut other = vec![100, 200];
    base.append(&mut other);
    println!("append: {:?} other={:?}", base, other);
    let drained: Vec<i32> = base.drain(1..4).collect();
    println!("drain: {:?} {:?}", drained, base);
    let removed: Vec<i32> = base.splice(1..3, [7, 7, 7]).collect();
    println!("splice: removed {:?} -> {:?}", removed, base);
    base.truncate(4);
    base.insert(1, -1);
    let sr = base.swap_remove(0);
    println!("truncate/insert/swap_remove: {} {:?}", sr, base);
    base.resize(7, 9);
    println!("resize: {:?}", base);
    let mut n = 0;
    base.resize_with(9, || { n += 1; n * 1000 });
    println!("resize_with: {:?}", base);
    base.extend_from_within(..2);
    println!("extend_from_within: {:?}", base);
    base.dedup();
    let popped = base.pop();
    println!("pop: {:?} len {} cap>=len {}", popped, base.len(), base.capacity() >= base.len());
    println!("pop_if: {:?} {:?}", base.pop_if(|x| *x > 5000), base.pop_if(|x| *x > 1));
    println!("contains/starts/ends: {} {} {}", base.contains(&9), base.starts_with(&[0]), base.ends_with(&[9]));
    println!("iter().rev().step_by(2): {:?}", base.iter().rev().step_by(2).collect::<Vec<_>>());
    println!("get: {:?} {:?} {:?}", base.get(1), base.get(1..3), base.get(5..99));
    let mut mm = vec![5, 6, 7];
    if let Some(x) = mm.get_mut(1) {
        *x += 100;
    }
    if let Some((first, rest)) = mm.split_first_mut() {
        *first = rest.iter().sum();
    }
    println!("get_mut/split_first_mut: {:?}", mm);
    let (l, r) = mm.split_at_mut(1);
    std::mem::swap(&mut l[0], &mut r[1]);
    println!("split_at_mut swap: {:?}", mm);
    println!("iter max/min/position: {:?} {:?} {:?} {:?}", mm.iter().max(), mm.iter().min(), mm.iter().position(|&x| x == 106), mm.iter().rposition(|&x| x > 0));
    let v2: Vec<String> = vec!["a".into(), "b".into()];
    let joined: String = v2.iter().map(String::as_str).collect::<Vec<_>>().join("+");
    println!("{} {:?}", joined, v2.into_iter().rev().collect::<Vec<_>>());
    let boxed: Box<[i32]> = vec![1, 2, 3].into_boxed_slice();
    println!("boxed slice: {:?} len {}", boxed, boxed.len());
    let back: Vec<i32> = boxed.into_vec();
    println!("{:?} {:?}", back, Vec::from([4, 5]));
    let mut cap = Vec::<u8>::with_capacity(10);
    cap.extend(b"abc");
    cap.shrink_to_fit();
    println!("{:?} {}", cap, String::from_utf8(cap.clone()).unwrap());
    let leaked_len = vec![0u16; 1000].iter().filter(|&&x| x == 0).count();
    println!("{}", leaked_len);
}
