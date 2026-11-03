// sort_by_key / sort_unstable_by / sort_by_cached_key / dedup_by_key / dedup_by / binary_search_by(_key) / partition_point
#[derive(Debug, Clone, PartialEq)]
struct Emp {
    name: &'static str,
    dept: &'static str,
    age: u32,
    salary: f64,
}

fn main() {
    let mut emps = vec![
        Emp { name: "ann", dept: "eng", age: 31, salary: 120.5 },
        Emp { name: "bob", dept: "ops", age: 45, salary: 99.0 },
        Emp { name: "cid", dept: "eng", age: 25, salary: 101.0 },
        Emp { name: "dee", dept: "hr", age: 38, salary: 87.25 },
        Emp { name: "eve", dept: "ops", age: 29, salary: 99.0 },
        Emp { name: "fay", dept: "eng", age: 31, salary: 150.0 },
    ];
    emps.sort_by_key(|e| e.age);
    println!("{:?}", emps.iter().map(|e| e.name).collect::<Vec<_>>());
    emps.sort_by_key(|e| std::cmp::Reverse(e.dept));
    println!("{:?}", emps.iter().map(|e| (e.dept, e.name)).collect::<Vec<_>>());
    emps.sort_by(|a, b| b.salary.partial_cmp(&a.salary).unwrap().then_with(|| a.name.cmp(b.name)));
    println!("{:?}", emps.iter().map(|e| e.name).collect::<Vec<_>>());
    emps.sort_unstable_by(|a, b| (a.dept, a.age, a.name).cmp(&(b.dept, b.age, b.name)));
    println!("{:?}", emps.iter().map(|e| format!("{}/{}/{}", e.dept, e.age, e.name)).collect::<Vec<_>>());
    let mut cached_calls = 0;
    emps.sort_by_cached_key(|e| {
        cached_calls += 1;
        e.name.chars().rev().collect::<String>()
    });
    println!("{:?} calls={}", emps.iter().map(|e| e.name).collect::<Vec<_>>(), cached_calls);
    let mut depts: Vec<&str> = emps.iter().map(|e| e.dept).collect();
    depts.sort_unstable();
    depts.dedup();
    println!("{:?}", depts);
    let mut by_dept = emps.clone();
    by_dept.sort_by(|a, b| a.dept.cmp(b.dept).then(b.salary.total_cmp(&a.salary)));
    by_dept.dedup_by_key(|e| e.dept);
    println!("top earners {:?}", by_dept.iter().map(|e| (e.dept, e.name)).collect::<Vec<_>>());
    let mut nums = vec![1, 1, 2, 3, 5, 8, 13, 21, 22, 23, 40];
    nums.dedup_by(|a, b| *a - *b <= 1);
    println!("{:?}", nums);
    let sorted = [1, 3, 5, 7, 9, 11, 13];
    println!("{:?} {:?}", sorted.binary_search(&7), sorted.binary_search(&8));
    println!("{:?}", sorted.binary_search_by(|x| x.cmp(&0)));
    println!("{:?}", sorted.binary_search_by(|x| x.cmp(&100)));
    let pairs = [(1, 'a'), (3, 'b'), (5, 'c'), (7, 'd')];
    println!("{:?} {:?}", pairs.binary_search_by_key(&5, |&(k, _)| k), pairs.binary_search_by_key(&4, |p| p.0));
    let pp = sorted.partition_point(|&x| x < 6);
    println!("pp={}", pp);
    let mut ins = vec![10, 20, 30];
    for x in [25, 5, 35, 20] {
        let i = ins.partition_point(|&y| y < x);
        ins.insert(i, x);
    }
    println!("{:?}", ins);
    let mut v = vec![5, -3, 2, -8, 7, 0, -1];
    v.sort_by_key(|x: &i32| x.abs());
    println!("{:?}", v);
    v.sort_unstable_by_key(|x| (x.signum(), -x));
    println!("{:?}", v);
    let mut strs = vec!["Banana", "apple", "cherry", "Apple", "banana"];
    strs.sort_by_key(|s| s.to_lowercase());
    println!("{:?}", strs);
    strs.sort();
    println!("{:?}", strs);
    strs.dedup_by(|a, b| a.eq_ignore_ascii_case(b));
    println!("{:?}", strs);
    let mut floats = vec![2.5, -0.5, 10.0, 3.25];
    floats.sort_by(|a: &f64, b| b.partial_cmp(a).unwrap());
    println!("{:?}", floats);
    println!("{}", [1, 2, 2, 3].is_sorted());
    println!("{}", [3, 2, 1].is_sorted_by(|a, b| a >= b));
    println!("{}", ["a", "bb", "c"].is_sorted_by_key(|s| s.len()));
    let mut big: Vec<u32> = (0..50).map(|i| (i * 37 + 11) % 50).collect();
    big.select_nth_unstable(10);
    println!("{}", big[10]);
    big.sort();
    println!("{:?}", &big[..5]);
    let mut words = vec!["delta", "alpha", "charlie", "bravo"];
    words.reverse();
    println!("{:?} {:?}", words, words.iter().max());
    let mut chunks = vec![[3, 1], [1, 2], [3, 0], [1, 1]];
    chunks.sort();
    println!("{:?}", chunks);
    println!("{:?}", emps.iter().position(|e| e.age > 40));
    println!("{}", emps.contains(&emps[2].clone()));
}
