// Text statistics over a multi-paragraph document: word frequencies (BTreeMap and HashMap sorted
// by count then word), bigrams, sentence splitting, longest words, per-letter histogram rendered with
// bars, BTreeMap range queries/entry API/retain/split_off/append, and word wrapping to a width.
use std::collections::{BTreeMap, HashMap};

const TEXT: &str = "The quick brown fox jumps over the lazy dog. The dog sleeps!
A fox is quick; a dog is lazy. Is the fox quicker than the dog?

Foxes and dogs: the eternal story. Quick thinking wins, lazy thinking loses.";

fn words(s: &str) -> impl Iterator<Item = String> + '_ {
    s.split(|c: char| !c.is_alphanumeric() && c != '\'').filter(|w| !w.is_empty()).map(|w| w.to_lowercase())
}

fn wrap(text: &str, width: usize) -> Vec<String> {
    let mut lines = vec![];
    let mut cur = String::new();
    for w in text.split_whitespace() {
        if !cur.is_empty() && cur.len() + 1 + w.len() > width {
            lines.push(std::mem::take(&mut cur));
        }
        if !cur.is_empty() {
            cur.push(' ');
        }
        cur.push_str(w);
    }
    if !cur.is_empty() {
        lines.push(cur);
    }
    lines
}

fn main() {
    let mut freq: BTreeMap<String, usize> = BTreeMap::new();
    for w in words(TEXT) {
        *freq.entry(w).or_insert(0) += 1;
    }
    println!("distinct={} total={}", freq.len(), freq.values().sum::<usize>());
    let mut by_count: Vec<(&String, &usize)> = freq.iter().collect();
    by_count.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
    for (w, c) in by_count.iter().take(6) {
        println!("{:>8} {}", w, c);
    }

    let mut hm: HashMap<(String, String), u32> = HashMap::new();
    let ws: Vec<String> = words(TEXT).collect();
    for pair in ws.windows(2) {
        *hm.entry((pair[0].clone(), pair[1].clone())).or_default() += 1;
    }
    let mut bigrams: Vec<_> = hm.into_iter().filter(|(_, c)| *c > 1).collect();
    bigrams.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    println!("bigrams {:?}", bigrams);

    let sentences: Vec<&str> = TEXT.split_inclusive(|c| matches!(c, '.' | '!' | '?')).map(str::trim).filter(|s| !s.is_empty()).collect();
    println!("{} sentences", sentences.len());
    for (i, s) in sentences.iter().enumerate() {
        println!("  {}: {} words, ends {:?}", i, words(s).count(), s.chars().last().unwrap());
    }
    let paragraphs = TEXT.split("\n\n").count();
    println!("paragraphs {}", paragraphs);

    let mut longest: Vec<&String> = freq.keys().collect();
    longest.sort_by_key(|w| (std::cmp::Reverse(w.len()), (*w).clone()));
    println!("longest {:?}", &longest[..3]);

    let mut letters: BTreeMap<char, usize> = BTreeMap::new();
    for c in TEXT.chars().filter(|c| c.is_alphabetic()) {
        *letters.entry(c.to_ascii_lowercase()).or_default() += 1;
    }
    let max = *letters.values().max().unwrap();
    for (c, n) in letters.range('a'..='h') {
        println!("{} {:>3} {}", c, n, "#".repeat(n * 30 / max));
    }
    let vowels: usize = letters.iter().filter(|(c, _)| "aeiou".contains(**c)).map(|(_, n)| n).sum();
    println!("vowels {} consonants {}", vowels, letters.values().sum::<usize>() - vowels);

    // BTreeMap API workout
    let mut m = freq.clone();
    m.retain(|_, c| *c >= 2);
    println!("retained {:?}", m);
    let tail = m.split_off("is");
    println!("head {:?}", m.keys().collect::<Vec<_>>());
    println!("tail {:?}", tail.keys().collect::<Vec<_>>());
    let mut joined = m;
    let mut t2 = tail;
    joined.append(&mut t2);
    println!("joined {} t2 {}", joined.len(), t2.len());
    println!("first {:?} last {:?}", joined.first_key_value(), joined.last_key_value());
    println!("range q.. {:?}", joined.range("q".to_string()..).map(|(k, _)| k.as_str()).collect::<Vec<_>>());
    println!("before 'f' {:?}", joined.range(..="f".to_string()).next_back());
    if let Some(v) = joined.get_mut("the") {
        *v *= 100;
    }
    let popped = joined.pop_last();
    println!("{:?} {:?}", popped, joined.get("the"));
    match joined.entry("zebra".to_string()) {
        std::collections::btree_map::Entry::Vacant(e) => {
            e.insert(1);
        }
        std::collections::btree_map::Entry::Occupied(_) => {}
    }
    println!("{:?}", joined.iter().rev().take(2).collect::<Vec<_>>());

    for (i, line) in wrap(TEXT, 28).iter().enumerate() {
        println!("{:>2}|{:<28}|", i, line);
    }
}
