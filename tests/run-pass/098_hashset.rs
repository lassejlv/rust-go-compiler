use std::collections::HashSet;

fn sorted<T: Ord + Clone>(s: &HashSet<T>) -> Vec<T> {
    let mut v: Vec<T> = s.iter().cloned().collect();
    v.sort();
    v
}

fn main() {
    let mut a: HashSet<i32> = HashSet::new();
    for x in [1, 2, 3, 4, 5, 3, 1] {
        let fresh = a.insert(x);
        if !fresh {
            println!("duplicate {}", x);
        }
    }
    println!("a = {:?} (len {})", sorted(&a), a.len());
    let b: HashSet<i32> = [4, 5, 6, 7].into_iter().collect();
    let mut u: Vec<i32> = a.union(&b).cloned().collect();
    u.sort();
    println!("union = {:?}", u);
    let mut i: Vec<&i32> = a.intersection(&b).collect();
    i.sort();
    println!("intersection = {:?}", i);
    let mut d: Vec<i32> = a.difference(&b).copied().collect();
    d.sort();
    println!("a - b = {:?}", d);
    let mut sd: Vec<i32> = a.symmetric_difference(&b).copied().collect();
    sd.sort();
    println!("symmetric diff = {:?}", sd);
    println!("contains 3: {}, contains 9: {}", a.contains(&3), a.contains(&9));
    println!("remove 3: {}, remove 3 again: {}", a.remove(&3), a.remove(&3));
    let small: HashSet<i32> = [4, 5].into_iter().collect();
    println!("small subset of a: {}, of b: {}", small.is_subset(&a), small.is_subset(&b));
    println!("b superset of small: {}", b.is_superset(&small));
    let disjoint: HashSet<i32> = [100].into_iter().collect();
    println!("disjoint: {}", a.is_disjoint(&disjoint));
    let words = "the cat and the hat and the bat";
    let unique: HashSet<&str> = words.split(' ').collect();
    println!("unique words: {:?}", sorted(&unique));
    let s1: HashSet<char> = "hello".chars().collect();
    let s2: HashSet<char> = "world".chars().collect();
    let common: HashSet<char> = &s1 & &s2;
    let all: HashSet<char> = &s1 | &s2;
    println!("common letters: {:?}", sorted(&common));
    println!("all letters: {:?}", sorted(&all));
    let mut seen = HashSet::new();
    let dupes: Vec<i32> = vec![3, 1, 4, 1, 5, 9, 2, 6, 5, 3, 5].into_iter().filter(|x| !seen.insert(*x)).collect();
    println!("dupes in order: {:?}", dupes);
    let points: HashSet<(i32, i32)> = [(0, 0), (1, 2), (0, 0), (2, 1)].into_iter().collect();
    println!("unique points: {:?}", sorted(&points));
    a.retain(|x| x % 2 == 0);
    println!("retain even: {:?}", sorted(&a));
    a.extend([10, 20]);
    println!("extended: {:?}", sorted(&a));
}
