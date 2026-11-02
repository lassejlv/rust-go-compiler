use std::collections::{BTreeMap, HashMap, HashSet};

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
enum Suit {
    Hearts,
    Spades,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
struct Card {
    rank: u8,
    suit: Suit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
enum Key {
    Id(u32),
    Pair(i8, i8),
    Named,
}

fn count_words(text: &str) -> HashMap<String, usize> {
    let mut m = HashMap::new();
    for w in text.split_whitespace() {
        *m.entry(w.to_lowercase()).or_insert(0) += 1;
    }
    m
}

fn main() {
    let counts = count_words("the cat The dog the end Dog");
    // Lookup String-keyed map with &str via Borrow.
    for w in ["the", "dog", "cat", "bird"] {
        println!("{} -> {:?}", w, counts.get(w));
    }
    println!("contains 'end': {}", counts.contains_key("end"));
    let owned = String::from("the");
    println!("by &String: {}", counts[&owned]);
    println!("by str index: {}", counts["dog"]);
    let mut sorted: Vec<(&String, &usize)> = counts.iter().collect();
    sorted.sort();
    println!("{:?}", sorted);

    let mut m2 = counts.clone();
    println!("removed {:?}", m2.remove("the"));
    println!("len {}", m2.len());
    if let Some(v) = m2.get_mut("cat") {
        *v += 100;
    }
    println!("cat {}", m2["cat"]);

    let mut hand: HashSet<Card> = HashSet::new();
    let cards = vec![
        Card { rank: 10, suit: Suit::Hearts },
        Card { rank: 2, suit: Suit::Spades },
        Card { rank: 10, suit: Suit::Hearts },
        Card { rank: 10, suit: Suit::Spades },
    ];
    for c in cards {
        let inserted = hand.insert(c.clone());
        println!("insert {:?} -> {}", c, inserted);
    }
    println!("hand size {}", hand.len());
    let mut v: Vec<_> = hand.into_iter().collect();
    v.sort();
    println!("{:?}", v);

    let mut by_key: HashMap<Key, Vec<&str>> = HashMap::new();
    let items = [(Key::Id(3), "a"), (Key::Pair(1, -1), "b"), (Key::Named, "c"), (Key::Id(3), "d"), (Key::Pair(1, -1), "e")];
    for (k, s) in items.iter() {
        by_key.entry(*k).or_default().push(s);
    }
    let mut keys: Vec<_> = by_key.keys().copied().collect();
    keys.sort();
    for k in keys {
        println!("{:?} => {:?}", k, by_key[&k]);
    }

    let mut tuple_map: HashMap<(i32, i32), char> = HashMap::new();
    tuple_map.insert((0, 0), 'o');
    tuple_map.insert((1, -1), 'x');
    println!("{:?} {:?}", tuple_map.get(&(1, -1)), tuple_map.get(&(2, 2)));

    let set_a: HashSet<&str> = ["x", "y", "z"].into_iter().collect();
    let set_b: HashSet<&str> = ["y", "z", "w"].into_iter().collect();
    let mut inter: Vec<_> = set_a.intersection(&set_b).cloned().collect();
    inter.sort();
    let mut uni: Vec<_> = set_a.union(&set_b).cloned().collect();
    uni.sort();
    let mut diff: Vec<_> = set_a.difference(&set_b).collect();
    diff.sort();
    println!("{:?} {:?} {:?}", inter, uni, diff);

    let bt: BTreeMap<String, i32> = [("b".to_string(), 2), ("a".to_string(), 1)].into_iter().collect();
    println!("{:?} {:?}", bt.get("a"), bt);
}
