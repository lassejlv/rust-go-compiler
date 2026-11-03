#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Rec { dept: &'static str, age: u32, name: String }

fn rec(dept: &'static str, age: u32, name: &str) -> Rec { Rec { dept, age, name: name.to_string() } }

fn main() {
    let mut v: Vec<i32> = vec![5, 3, 8, 1, 9, 3, 3, 7, 1, 0, 8, 6];
    v.sort_unstable();
    println!("sorted {:?}", v);
    for t in [3, 4, 9, 10, -1] {
        println!("binary_search({}) = {:?}  partition_point(<{}) = {}", t, v.binary_search(&t).map(|_| "found"), t, v.partition_point(|&x| x < t));
    }
    v.dedup();
    println!("dedup {:?}", v);
    let mut w = vec![1, 1, 2, 3, 3, 3, 4, 1, 1];
    w.dedup_by_key(|x| *x / 2);
    println!("dedup_by_key {:?}", w);
    let mut names = vec!["apple", "Apple", "APPLE", "banana", "Banana", "cherry"];
    names.dedup_by(|a, b| a.eq_ignore_ascii_case(b));
    println!("dedup_by {:?}", names);

    let mut r: Vec<i32> = (1..=20).collect();
    r.retain(|&x| x % 3 != 0);
    println!("retain {:?}", r);
    let mut counter = 0;
    r.retain_mut(|x| { counter += 1; *x *= 2; counter % 2 == 0 });
    println!("retain_mut {:?}", r);
    let drained: Vec<i32> = r.drain(1..4).collect();
    println!("drained {:?} left {:?}", drained, r);
    let tail: Vec<i32> = r.drain(r.len() - 2..).rev().collect();
    println!("tail {:?} left {:?}", tail, r);
    let removed: Vec<i32> = r.splice(0..2, [100, 200, 300]).collect();
    println!("splice removed {:?} now {:?}", removed, r);
    let _ = r.splice(2..2, vec![-1, -2]);
    println!("splice insert {:?}", r);
    r.truncate(5);
    r.insert(1, 42);
    let x = r.remove(3);
    let y = r.swap_remove(0);
    println!("trunc/insert/remove {:?} x={} y={}", r, x, y);
    let mut ex = vec![1, 2, 3];
    ex.extend_from_slice(&[4, 5]);
    ex.extend([6, 7].iter());
    let mut other = vec![8, 9];
    ex.append(&mut other);
    println!("extend {:?} other={:?}", ex, other);
    let split = ex.split_off(6);
    println!("split_off {:?} {:?}", ex, split);
    ex.rotate_left(2);
    println!("rotl {:?}", ex);
    ex.rotate_right(4);
    println!("rotr {:?}", ex);
    ex.reverse();
    ex.swap(0, 5);
    println!("rev+swap {:?}", ex);
    ex.fill(0);
    ex[1..3].copy_from_slice(&[7, 7]);
    println!("fill/copy {:?}", ex);
    let mut gen = 1;
    ex.fill_with(|| { gen *= 3; gen });
    println!("fill_with {:?}", ex);
    println!("contains {} starts_with {} ends_with {}", ex.contains(&27), ex.starts_with(&[3, 9]), ex.ends_with(&[729]));
    println!("first {:?} last {:?} get(10) {:?} split_first {:?}", ex.first(), ex.last(), ex.get(10), ex.split_first().map(|(a, b)| (*a, b.len())));
    let (l, rr) = ex.split_at(2);
    println!("split_at {:?} {:?}", l, rr);
    let parts: Vec<&[i32]> = ex.split(|x| x % 9 == 0).collect();
    println!("split by pred {:?}", parts);
    println!("concat {:?} join {:?}", [[1, 2], [3, 4]].concat(), [vec![1], vec![2, 3]].join(&0));
    println!("repeat {:?}", [1, 2].repeat(3));

    let mut recs = vec![rec("eng", 30, "zed"), rec("ops", 25, "amy"), rec("eng", 25, "bob"), rec("ops", 25, "al"), rec("eng", 30, "cat"), rec("hr", 41, "dan")];
    recs.sort_by_key(|r| r.age);
    println!("by age (stable): {:?}", recs.iter().map(|r| r.name.as_str()).collect::<Vec<_>>());
    recs.sort_by(|a, b| a.dept.cmp(b.dept).then_with(|| b.age.cmp(&a.age)));
    println!("by dept, age desc: {:?}", recs.iter().map(|r| format!("{}/{}/{}", r.dept, r.age, r.name)).collect::<Vec<_>>());
    recs.sort();
    println!("derived Ord: {:?}", recs.iter().map(|r| &r.name).collect::<Vec<_>>());
    recs.sort_by_cached_key(|r| r.name.len());
    println!("cached key: {:?}", recs.iter().map(|r| &r.name).collect::<Vec<_>>());
    let pos = recs.binary_search_by(|r| r.name.len().cmp(&3));
    println!("bsearch_by len3 ok? {}", pos.is_ok());
    let pos2 = recs.binary_search_by_key(&4, |r| r.name.len());
    println!("bsearch_by_key len4 {:?}", pos2);
    let mut nums = vec![9, 4, 7, 1, 8, 2, 6];
    let (lo, mid, hi) = nums.select_nth_unstable(3);
    let mut lo = lo.to_vec(); lo.sort();
    let mut hi = hi.to_vec(); hi.sort();
    println!("select_nth: {:?} {} {:?}", lo, mid, hi);
    nums.sort_unstable_by(|a, b| b.cmp(a));
    println!("desc {:?} is_sorted_by {}", nums, nums.windows(2).all(|w| w[0] >= w[1]));
    let mut chunks: Vec<i32> = (1..=10).collect();
    for c in chunks.chunks_mut(3) { c.reverse(); }
    println!("chunks_mut rev {:?}", chunks);
    let mut it = chunks.iter_mut().rev().step_by(2);
    while let Some(x) = it.next() { *x = -*x; }
    println!("iter_mut rev step {:?}", chunks);
    let mut vv = vec![vec![3, 1], vec![], vec![2]];
    vv.sort_by_key(|v| std::cmp::Reverse(v.len()));
    println!("{:?} max={:?} min={:?}", vv, vv.iter().max(), vv.iter().min());
    let mut dq = vec![1, 2, 3, 4, 5, 6, 7, 8];
    let ex2: Vec<i32> = dq.extract_if(.., |x| *x % 3 == 0).collect();
    println!("extract_if {:?} {:?}", ex2, dq);
}
