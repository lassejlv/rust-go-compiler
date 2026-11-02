use std::collections::{BTreeMap, HashMap, HashSet};

const TEXT: &str = "It was the best of times, it was the worst of times, it was the age of wisdom, \
it was the age of foolishness, it was the epoch of belief, it was the epoch of incredulity, \
it was the season of Light, it was the season of Darkness, it was the spring of hope, \
it was the winter of despair, we had everything before us, we had nothing before us.";

fn tokenize(text: &str) -> Vec<String> {
    text.split(|c: char| !c.is_alphabetic() && c != '\'')
        .filter(|w| !w.is_empty())
        .map(|w| w.to_lowercase())
        .collect()
}

fn main() {
    let words = tokenize(TEXT);
    println!("total words: {}", words.len());
    let mut freq: HashMap<&str, usize> = HashMap::new();
    for w in &words {
        *freq.entry(w.as_str()).or_insert(0) += 1;
    }
    println!("distinct words: {}", freq.len());
    let mut ranked: Vec<(&str, usize)> = freq.iter().map(|(k, v)| (*k, *v)).collect();
    ranked.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));
    println!("top 8:");
    for (i, (w, c)) in ranked.iter().take(8).enumerate() {
        println!("{:>2}. {:<8} {:>3} {}", i + 1, w, c, "*".repeat(*c));
    }
    let singles: Vec<&str> = {
        let mut s: Vec<&str> = ranked.iter().filter(|(_, c)| *c == 1).map(|(w, _)| *w).collect();
        s.sort();
        s
    };
    println!("hapax legomena ({}): {}", singles.len(), singles.join(" "));
    let mut by_len: BTreeMap<usize, HashSet<&str>> = BTreeMap::new();
    for w in &words {
        by_len.entry(w.len()).or_default().insert(w.as_str());
    }
    for (len, set) in &by_len {
        let mut ws: Vec<&&str> = set.iter().collect();
        ws.sort();
        println!("len {:>2}: {:?}", len, ws);
    }
    let mut bigrams: HashMap<(String, String), usize> = HashMap::new();
    for pair in words.windows(2) {
        *bigrams.entry((pair[0].clone(), pair[1].clone())).or_insert(0) += 1;
    }
    let mut top_bi: Vec<_> = bigrams.into_iter().filter(|(_, c)| *c > 2).collect();
    top_bi.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    for ((a, b), c) in &top_bi {
        println!("bigram '{} {}' x{}", a, b, c);
    }
    let letters: BTreeMap<char, usize> = words.iter().flat_map(|w| w.chars()).fold(BTreeMap::new(), |mut m, c| {
        *m.entry(c).or_insert(0) += 1;
        m
    });
    let top_letters: Vec<String> = {
        let mut v: Vec<(&char, &usize)> = letters.iter().collect();
        v.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
        v.iter().take(5).map(|(c, n)| format!("{}:{}", c, n)).collect()
    };
    println!("top letters: {}", top_letters.join(" "));
    let avg = words.iter().map(|w| w.len()).sum::<usize>() as f64 / words.len() as f64;
    println!("average word length: {:.3}", avg);
}
