use std::collections::HashMap;

fn main() {
    let mut scores: HashMap<String, i32> = HashMap::new();
    scores.insert("blue".to_string(), 10);
    scores.insert("red".to_string(), 50);
    scores.insert("green".to_string(), 30);
    println!("len = {}", scores.len());
    println!("get blue = {:?}", scores.get("blue"));
    println!("get pink = {:?}", scores.get("pink"));
    println!("contains red = {}", scores.contains_key("red"));
    let old = scores.insert("blue".to_string(), 25);
    println!("replaced old blue = {:?}, new = {}", old, scores["blue"]);
    if let Some(v) = scores.get_mut("green") {
        *v += 5;
    }
    println!("green = {}", scores["green"]);
    let removed = scores.remove("red");
    println!("removed red = {:?}, len = {}", removed, scores.len());
    println!("remove missing = {:?}", scores.remove("red"));
    scores.insert("yellow".to_string(), 1);
    let mut keys: Vec<&String> = scores.keys().collect();
    keys.sort();
    println!("keys = {:?}", keys);
    let mut vals: Vec<i32> = scores.values().cloned().collect();
    vals.sort();
    println!("values = {:?}", vals);
    let mut pairs: Vec<(&String, &i32)> = scores.iter().collect();
    pairs.sort_by(|a, b| b.1.cmp(a.1));
    for (k, v) in &pairs {
        println!("  {} -> {}", k, v);
    }
    let total: i32 = scores.values().sum();
    println!("total = {}", total);
    for v in scores.values_mut() {
        *v *= 2;
    }
    let mut doubled: Vec<(String, i32)> = scores.clone().into_iter().collect();
    doubled.sort();
    println!("doubled = {:?}", doubled);
    let mut ids: HashMap<u32, &str> = HashMap::new();
    for (i, name) in ["zero", "one", "two", "three"].iter().enumerate() {
        ids.insert(i as u32, name);
    }
    for i in [3, 0, 7] {
        match ids.get(&i) {
            Some(n) => println!("id {} = {}", i, n),
            None => println!("id {} missing", i),
        }
    }
    ids.retain(|k, _| k % 2 == 1);
    let mut left: Vec<_> = ids.into_iter().collect();
    left.sort();
    println!("after retain = {:?}", left);
    let mut tuple_keys: HashMap<(i32, i32), char> = HashMap::new();
    tuple_keys.insert((0, 0), 'o');
    tuple_keys.insert((1, -1), 'x');
    println!("tuple key lookup = {:?} {:?}", tuple_keys.get(&(1, -1)), tuple_keys.get(&(2, 2)));
    println!("is_empty = {}", HashMap::<i32, i32>::new().is_empty());
    let m2 = HashMap::from([("a", 1), ("b", 2)]);
    let mut m2v: Vec<_> = m2.iter().map(|(k, v)| format!("{}={}", k, v)).collect();
    m2v.sort();
    println!("from array = {}", m2v.join(","));
}
