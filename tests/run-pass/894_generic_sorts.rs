use std::fmt::Debug;
use std::cmp::Ordering;

fn quicksort<T: PartialOrd + Clone>(v: &mut [T]) {
    if v.len() <= 1 { return; }
    let p = partition(v);
    let (l, r) = v.split_at_mut(p);
    quicksort(l);
    quicksort(&mut r[1..]);
}
fn partition<T: PartialOrd + Clone>(v: &mut [T]) -> usize {
    let last = v.len() - 1;
    let pivot = v[last].clone();
    let mut i = 0;
    for j in 0..last {
        if v[j] <= pivot { v.swap(i, j); i += 1; }
    }
    v.swap(i, last);
    i
}

fn merge_sort_by<T: Clone, F: Fn(&T, &T) -> Ordering + Copy>(v: &[T], cmp: F) -> Vec<T> {
    if v.len() <= 1 { return v.to_vec(); }
    let mid = v.len() / 2;
    let l = merge_sort_by(&v[..mid], cmp);
    let r = merge_sort_by(&v[mid..], cmp);
    let (mut i, mut j) = (0, 0);
    let mut out = Vec::with_capacity(v.len());
    while i < l.len() && j < r.len() {
        if cmp(&l[i], &r[j]) != Ordering::Greater { out.push(l[i].clone()); i += 1; } else { out.push(r[j].clone()); j += 1; }
    }
    out.extend_from_slice(&l[i..]);
    out.extend_from_slice(&r[j..]);
    out
}

fn insertion_sort_by_key<T, K: Ord, F: Fn(&T) -> K>(v: &mut Vec<T>, key: F) {
    for i in 1..v.len() {
        let mut j = i;
        while j > 0 && key(&v[j - 1]) > key(&v[j]) { v.swap(j - 1, j); j -= 1; }
    }
}

fn binary_search_by<T, F: Fn(&T) -> Ordering>(v: &[T], f: F) -> Result<usize, usize> {
    let (mut lo, mut hi) = (0, v.len());
    while lo < hi {
        let mid = lo + (hi - lo) / 2;
        match f(&v[mid]) {
            Ordering::Less => lo = mid + 1,
            Ordering::Greater => hi = mid,
            Ordering::Equal => return Ok(mid),
        }
    }
    Err(lo)
}

fn lower_bound<T: Ord>(v: &[T], x: &T) -> usize {
    let (mut lo, mut hi) = (0, v.len());
    while lo < hi { let m = (lo + hi) / 2; if v[m] < *x { lo = m + 1 } else { hi = m } }
    lo
}

fn is_sorted<T: PartialOrd>(v: &[T]) -> bool { v.windows(2).all(|w| w[0] <= w[1]) }

fn kth_smallest<T: Ord + Clone + Debug>(v: &[T], k: usize) -> T {
    let mut c = v.to_vec();
    quicksort(&mut c);
    c[k].clone()
}

#[derive(Debug, Clone, PartialEq, PartialOrd)]
struct Emp { dept: &'static str, salary: f64, name: &'static str }

fn main() {
    let mut a = vec![5, 3, 9, 1, 5, 8, 2, 7, -4];
    quicksort(&mut a);
    println!("{:?} {}", a, is_sorted(&a));
    let mut f = vec![2.5, -1.0, 3.75, 0.0];
    quicksort(&mut f);
    println!("{:?}", f);
    let mut s: Vec<String> = "pear apple fig banana cherry".split(' ').map(String::from).collect();
    quicksort(&mut s);
    println!("{:?}", s);
    let emps = vec![
        Emp { dept: "eng", salary: 120.0, name: "a" },
        Emp { dept: "ops", salary: 90.0, name: "b" },
        Emp { dept: "eng", salary: 100.0, name: "c" },
        Emp { dept: "ops", salary: 90.0, name: "d" },
        Emp { dept: "hr", salary: 95.0, name: "e" },
    ];
    let by = merge_sort_by(&emps, |x, y| x.dept.cmp(y.dept).then(y.salary.partial_cmp(&x.salary).unwrap()));
    for e in &by { println!("{} {} {}", e.dept, e.salary, e.name); }
    let names: Vec<_> = merge_sort_by(&emps, |x, y| x.salary.partial_cmp(&y.salary).unwrap()).iter().map(|e| e.name).collect();
    println!("{:?}", names);
    let mut words = vec!["ccc", "a", "bb", "dddd", "ee"];
    insertion_sort_by_key(&mut words, |w| w.len());
    println!("{:?}", words);
    insertion_sort_by_key(&mut words, |w| std::cmp::Reverse(w.chars().next().unwrap()));
    println!("{:?}", words);
    println!("{:?} {:?}", binary_search_by(&a, |x| x.cmp(&7)), binary_search_by(&a, |x| x.cmp(&6)));
    println!("{:?}", binary_search_by(&s, |x| x.as_str().cmp("fig")));
    println!("{} {} {}", lower_bound(&a, &5), lower_bound(&a, &100), lower_bound(&a, &-10));
    println!("{} {}", kth_smallest(&[9, 1, 8, 2, 7], 2), kth_smallest(&["z", "b", "m"], 0));
}
