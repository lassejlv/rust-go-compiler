#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Rec {
    dept: &'static str,
    age: u32,
    name: String,
}

fn r(dept: &'static str, age: u32, name: &str) -> Rec {
    Rec { dept, age, name: name.to_string() }
}

fn main() {
    let mut recs = vec![
        r("eng", 30, "ann"),
        r("ops", 25, "bob"),
        r("eng", 25, "cat"),
        r("hr", 40, "dan"),
        r("ops", 30, "eve"),
        r("eng", 30, "fay"),
    ];
    recs.sort_by_key(|x| x.age);
    println!("{:?}", recs.iter().map(|x| &x.name[..]).collect::<Vec<_>>());
    recs.sort_by_key(|x| std::cmp::Reverse(x.dept));
    println!("{:?}", recs.iter().map(|x| &x.name[..]).collect::<Vec<_>>());
    recs.sort_by(|a, b| a.dept.cmp(b.dept).then_with(|| b.age.cmp(&a.age)).then(a.name.cmp(&b.name)));
    println!("{:?}", recs.iter().map(|x| &x.name[..]).collect::<Vec<_>>());
    recs.sort_by_cached_key(|x| x.name.len() * 100 + x.age as usize);
    println!("{:?}", recs.iter().map(|x| &x.name[..]).collect::<Vec<_>>());
    recs.sort();
    println!("{:?}", recs.first());

    let mut nums = vec![5, 3, 9, 1, 3, 7, 2, 8, 3, 0];
    nums.sort_unstable();
    println!("{:?}", nums);
    nums.sort_unstable_by(|a, b| b.cmp(a));
    println!("{:?}", nums);
    nums.reverse();
    println!("{:?}", nums.binary_search(&3).is_ok());
    println!("{:?} {:?}", nums.binary_search(&4), nums.binary_search(&100));
    println!("{:?}", nums.binary_search_by(|p| p.cmp(&7)));
    println!("{:?}", nums.binary_search_by_key(&16, |&x| x * 2));
    println!("{}", nums.partition_point(|&x| x < 5));
    nums.dedup();
    println!("{:?}", nums);
    let mut words = vec!["apple", "avocado", "banana", "blueberry", "cherry", "apricot"];
    words.dedup_by_key(|w| w.chars().next());
    println!("{:?}", words);
    let mut ws = vec!["a", "A", "b", "B", "b", "c"];
    ws.dedup_by(|a, b| a.eq_ignore_ascii_case(b));
    println!("{:?}", ws);

    let mut v: Vec<i32> = (1..=10).collect();
    v.retain(|&x| x % 3 != 0);
    println!("{:?}", v);
    v.retain_mut(|x| {
        *x *= 2;
        *x < 15
    });
    println!("{:?}", v);
    let mut v: Vec<i32> = (0..10).collect();
    let d: Vec<i32> = v.drain(2..5).collect();
    println!("{:?} {:?}", d, v);
    let d: Vec<i32> = v.drain(..=1).rev().collect();
    println!("{:?} {:?}", d, v);
    let d: Vec<i32> = v.drain(3..).collect();
    println!("{:?} {:?}", d, v);
    {
        let mut dr = v.drain(..);
        println!("{:?} {:?}", dr.next(), dr.next_back());
    }
    println!("{:?}", v);
    let mut v: Vec<i32> = (0..8).collect();
    let tail = v.split_off(5);
    println!("{:?} {:?}", v, tail);
    let removed: Vec<i32> = v.splice(1..3, [10, 20, 30]).collect();
    println!("{:?} {:?}", v, removed);
    v.truncate(4);
    v.insert(1, -1);
    let r0 = v.remove(0);
    let r1 = v.swap_remove(0);
    println!("{:?} {} {}", v, r0, r1);
    println!("{:?}", v);
    v.extend_from_slice(&[7, 8, 9]);
    v.rotate_left(2);
    println!("{:?}", v);
    v.rotate_right(3);
    println!("{:?}", v);
    v.swap(0, 1);
    v[1..4].reverse();
    println!("{:?}", v);
    v.fill(0);
    println!("{:?}", v);
    let mut c = 0;
    v.fill_with(|| { c += 1; c * c });
    println!("{:?}", v);
    let (l, r) = v.split_at_mut(2);
    l[0] += r[0];
    r[1] = l[1];
    println!("{:?}", v);
    let mut arr = [1, 2, 3, 4, 5, 6];
    arr.copy_within(0..3, 3);
    println!("{:?}", arr);
    let mut dst = [0; 3];
    dst.copy_from_slice(&arr[1..4]);
    dst.clone_from_slice(&[9, 9, 9][..]);
    println!("{:?}", dst);
    println!("{:?} {:?}", arr.split_first(), arr.split_last());
    println!("{:?} {:?}", arr.first_mut().map(|x| { *x = 50; *x }), arr.last());
    println!("{} {} {}", arr.contains(&5), arr.starts_with(&[50, 2]), arr.ends_with(&[3]));
    println!("{:?}", arr.iter().rev().skip_while(|&&x| x > 2).collect::<Vec<_>>());
    println!("{:?}", [[1, 2], [3, 4]].concat());
    println!("{:?}", [vec![1], vec![2, 3]].join(&0));
    println!("{:?}", ["a", "b"].join("-"));
    println!("{:?}", vec![3; 4].repeat(2).len());
    let mut m = [5, 1, 4, 2, 3];
    m.select_nth_unstable(2);
    println!("{}", m[2]);
    let mut m2 = vec![1, 2, 3, 4, 5, 6];
    let (a, b) = m2.split_at(4);
    println!("{:?} {:?}", a, b);
    m2.chunks_mut(4).for_each(|c| c.reverse());
    println!("{:?}", m2);
    for w in m2.chunks_exact_mut(2) {
        w.swap(0, 1);
    }
    println!("{:?}", m2);
    println!("{:?}", m2.iter().max());
    println!("{:?}", m2.get(1..3));
    println!("{:?}", m2.get(5..9));
    if let Some(x) = m2.get_mut(0) {
        *x = 99;
    }
    println!("{:?}", m2.last_mut());
    let mut sorted = m2.clone();
    sorted.sort();
    println!("{} {}", sorted.is_sorted(), m2.is_sorted_by_key(|x| -x));
    println!("{:?}", m2.iter().copied().rev().collect::<Vec<_>>().as_slice() == &[5, 6, 4, 1, 3, 99][..]);
    let pos = m2.iter().position(|&x| x == 4);
    println!("{:?}", pos);
    let vv: Vec<Vec<u8>> = vec![vec![1, 2], vec![], vec![3]];
    let flat: Vec<u8> = vv.into_iter().flatten().collect();
    println!("{:?}", flat);
    let mut dq = vec![1, 2, 3];
    dq.dedup_by_key(|x| *x / 2);
    println!("{:?}", dq);
    let mut e: Vec<String> = Vec::with_capacity(3);
    e.push("x".into());
    e.resize(3, "y".into());
    e.resize_with(5, Default::default);
    println!("{:?} {}", e, e.capacity() >= 5);
    println!("{:?}", e.pop());
    e.clear();
    println!("{} {}", e.is_empty(), e.len());
}
