use std::collections::HashMap;

fn word_freq(text: &str) -> Vec<(String, usize)> {
    let mut m: HashMap<String, usize> = HashMap::new();
    for w in text.split(|c: char| !c.is_alphanumeric()).filter(|w| !w.is_empty()) {
        *m.entry(w.to_lowercase()).or_insert(0) += 1;
    }
    let mut v: Vec<(String, usize)> = m.into_iter().collect();
    v.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    v
}

fn main() {
    let text = "The quick brown fox. The lazy dog! The fox jumps; a dog sleeps, the end.";
    for (w, c) in word_freq(text).iter().take(5) {
        println!("{:>6}: {}", w, c);
    }
    let mut groups: HashMap<usize, Vec<&str>> = HashMap::new();
    for w in ["hi", "hey", "yo", "hello", "sup", "howdy"] {
        groups.entry(w.len()).or_insert_with(Vec::new).push(w);
    }
    let mut lens: Vec<_> = groups.keys().copied().collect();
    lens.sort();
    for l in lens {
        println!("len {}: {:?}", l, groups[&l]);
    }
    let mut stock: HashMap<&str, i32> = HashMap::new();
    let ops = [("apple", 5), ("pear", 2), ("apple", -3), ("kiwi", 7), ("pear", -2), ("apple", 10)];
    for (item, delta) in ops {
        stock.entry(item).and_modify(|q| *q += delta).or_insert(delta);
    }
    let mut items: Vec<_> = stock.iter().collect();
    items.sort();
    println!("stock: {:?}", items);
    let mut counts: HashMap<char, u32> = HashMap::new();
    for c in "abracadabra".chars() {
        *counts.entry(c).or_default() += 1;
    }
    let mut cv: Vec<(char, u32)> = counts.into_iter().collect();
    cv.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    println!("char counts: {:?}", cv);
    let mut cache: HashMap<u64, u64> = HashMap::new();
    fn collatz_len(n: u64, cache: &mut HashMap<u64, u64>) -> u64 {
        if n == 1 {
            return 1;
        }
        if let Some(&v) = cache.get(&n) {
            return v;
        }
        let next = if n % 2 == 0 { n / 2 } else { 3 * n + 1 };
        let r = 1 + collatz_len(next, cache);
        cache.insert(n, r);
        r
    }
    let mut best = (0, 0);
    for n in 1..10000 {
        let l = collatz_len(n, &mut cache);
        if l > best.1 {
            best = (n, l);
        }
    }
    println!("longest collatz under 10000: {:?}", best);
    let e = cache.entry(27).or_insert(0);
    println!("cached 27 -> {}", e);
    let mut inverted: HashMap<i32, Vec<&str>> = HashMap::new();
    let grades = [("amy", 90), ("bo", 85), ("cy", 90), ("di", 70), ("ed", 85)];
    for (name, g) in grades.iter() {
        inverted.entry(*g).or_default().push(name);
    }
    let mut gs: Vec<_> = inverted.into_iter().collect();
    gs.sort_by(|a, b| b.0.cmp(&a.0));
    println!("by grade: {:?}", gs);
}
