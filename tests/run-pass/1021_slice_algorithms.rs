use std::cmp::Ordering;
use std::cell::Cell;

#[derive(Debug, Clone)]
struct Emp { name: &'static str, dept: &'static str, salary: u32 }

fn main() {
    let calls = Cell::new(0);
    let mut words = vec!["banana", "Apple", "cherry", "apple", "Banana", "date"];
    words.sort_by_cached_key(|w| { calls.set(calls.get() + 1); w.to_lowercase() });
    println!("{:?} key calls={}", words, calls.get());
    let mut emps = vec![
        Emp { name: "ann", dept: "eng", salary: 120 },
        Emp { name: "bob", dept: "ops", salary: 90 },
        Emp { name: "cat", dept: "eng", salary: 150 },
        Emp { name: "dan", dept: "ops", salary: 90 },
        Emp { name: "eve", dept: "hr", salary: 100 },
    ];
    emps.sort_by(|a, b| a.dept.cmp(b.dept).then_with(|| b.salary.cmp(&a.salary)).then(a.name.cmp(b.name)));
    for e in &emps { println!("{} {} {}", e.dept, e.salary, e.name); }
    emps.sort_by_key(|e| std::cmp::Reverse(e.salary));
    println!("{:?}", emps.iter().map(|e| e.name).collect::<Vec<_>>());

    let sorted = [1, 3, 5, 7, 9, 11];
    for t in [7, 8, 0, 12] {
        println!("search {} -> {:?} / by {:?}", t, sorted.binary_search(&t),
            sorted.binary_search_by(|p| p.cmp(&t)));
    }
    let pairs = [(1, 'a'), (3, 'b'), (5, 'c')];
    println!("{:?}", pairs.binary_search_by_key(&3, |&(k, _)| k));
    let fs = [0.5, 1.5, 2.5];
    println!("{:?}", fs.binary_search_by(|p| p.partial_cmp(&2.0).unwrap()));
    println!("partition_point {}", sorted.partition_point(|&x| x < 6));

    let data: Vec<u32> = (1..=11).collect();
    let ce = data.chunks_exact(3);
    println!("remainder {:?}", ce.remainder());
    println!("{:?}", ce.map(|c| c.iter().sum::<u32>()).collect::<Vec<_>>());
    println!("{:?}", data.rchunks(4).collect::<Vec<_>>());
    println!("{:?}", data.rchunks_exact(4).collect::<Vec<_>>());
    println!("{:?}", data.windows(4).filter(|w| w[0] % 3 == 0).collect::<Vec<_>>());
    let mut m = data.clone();
    for c in m.chunks_exact_mut(2) { c.swap(0, 1); }
    println!("{:?}", m);
    for c in m.rchunks_mut(5) { c.reverse(); }
    println!("{:?}", m);
    println!("{:?}", data.chunk_by(|a, b| b - a == 1 && a % 4 != 0).collect::<Vec<_>>());
    println!("{:?}", data.split(|x| x % 4 == 0).collect::<Vec<_>>());
    println!("{:?}", data.rsplitn(3, |x| x % 3 == 0).collect::<Vec<_>>());
    let (l, r) = data.split_at(4);
    println!("{:?} {:?}", l, r);
    println!("{:?} {:?}", data.first_chunk::<2>(), data.last_chunk::<3>());
    let mut rot = [1, 2, 3, 4, 5];
    rot.rotate_left(2);
    println!("{:?}", rot);
    rot.rotate_right(1);
    println!("{:?}", rot);
    let mut v = vec![5, 1, 4, 1, 5, 9, 2, 6];
    v.select_nth_unstable(3);
    println!("nth3 = {}", v[3]);
    v.sort_unstable_by(|a, b| b.cmp(a));
    v.dedup();
    println!("{:?}", v);
    println!("{:?} {:?}", v.iter().max_by(|a, b| (*a % 5).cmp(&(*b % 5))), v.iter().min_by_key(|x| (**x as i32 - 4).abs()));
    println!("{}", [1, 2, 3].cmp(&[1, 2, 4]) == Ordering::Less);
    println!("{:?}", [3, 1, 2].iter().is_sorted());
    println!("{}", data.contains(&7) && data.starts_with(&[1, 2]) && data.ends_with(&[11]));
    let mut fill = [0; 5];
    fill.fill_with({ let mut n = 0; move || { n += 2; n } });
    println!("{:?}", fill);
    println!("{:?}", [[1, 2], [3, 4]].concat());
    println!("{:?}", [vec![1], vec![2, 3]].join(&0));
    println!("{:?}", ["a", "b"].join("-"));
}
