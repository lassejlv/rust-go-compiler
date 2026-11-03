// text processing: word frequency, deterministic sorting, n-grams, line wrapping, justify
use std::collections::{BTreeMap, HashMap};

const TEXT: &str = "It was the best of times, it was the worst of times, it was the age of wisdom, \
it was the age of foolishness, it was the epoch of belief, it was the epoch of incredulity, \
it was the season of Light, it was the season of Darkness, it was the spring of hope, \
it was the winter of despair.";

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

fn justify(line: &str, width: usize) -> String {
    let ws: Vec<&str> = line.split(' ').collect();
    if ws.len() == 1 {
        return format!("{:<width$}", line, width = width);
    }
    let letters: usize = ws.iter().map(|w| w.len()).sum();
    let gaps = ws.len() - 1;
    let spaces = width - letters;
    let mut out = String::new();
    for (i, w) in ws.iter().enumerate() {
        out.push_str(w);
        if i < gaps {
            let n = spaces / gaps + if i < spaces % gaps { 1 } else { 0 };
            out.push_str(&" ".repeat(n));
        }
    }
    out
}

fn main() {
    let mut freq: HashMap<String, usize> = HashMap::new();
    for w in words(TEXT) {
        *freq.entry(w).or_insert(0) += 1;
    }
    let mut top: Vec<(&String, &usize)> = freq.iter().collect();
    top.sort_by(|a, b| b.1.cmp(a.1).then_with(|| a.0.cmp(b.0)));
    for (w, c) in top.iter().take(8) {
        println!("{:<8} {:>3} {}", w, c, "#".repeat(**c));
    }
    println!("distinct={} total={}", freq.len(), freq.values().sum::<usize>());
    let singles: Vec<&str> = {
        let mut v: Vec<&str> = freq.iter().filter(|(_, &c)| c == 1).map(|(w, _)| w.as_str()).collect();
        v.sort_unstable();
        v
    };
    println!("{:?}", singles);
    let ws: Vec<String> = words(TEXT).collect();
    let mut bigrams: BTreeMap<(String, String), usize> = BTreeMap::new();
    for pair in ws.windows(2) {
        *bigrams.entry((pair[0].clone(), pair[1].clone())).or_default() += 1;
    }
    let mut bg: Vec<_> = bigrams.into_iter().filter(|(_, c)| *c > 2).collect();
    bg.sort_by_key(|(k, c)| (std::cmp::Reverse(*c), k.clone()));
    println!("{:?}", bg);
    let by_len: BTreeMap<usize, usize> = ws.iter().fold(BTreeMap::new(), |mut m, w| {
        *m.entry(w.chars().count()).or_insert(0) += 1;
        m
    });
    println!("{:?}", by_len);
    let first_letters: BTreeMap<char, Vec<&str>> = {
        let mut m: BTreeMap<char, Vec<&str>> = BTreeMap::new();
        for w in freq.keys() {
            m.entry(w.chars().next().unwrap()).or_default().push(w);
        }
        m.values_mut().for_each(|v| v.sort());
        m
    };
    for (c, v) in first_letters.iter().filter(|(_, v)| v.len() > 1) {
        println!("{}: {}", c, v.join(","));
    }
    for line in wrap(TEXT, 40).iter().take(5) {
        println!("|{}|", justify(line, 40));
    }
    let longest = ws.iter().max_by(|a, b| a.len().cmp(&b.len()).then(b.cmp(a))).unwrap();
    println!("longest {}", longest);
    let caps: Vec<&str> = TEXT.split_whitespace().filter(|w| w.chars().next().map_or(false, |c| c.is_uppercase())).collect();
    println!("{:?}", caps);
    let sentence_count = TEXT.matches(|c| c == '.' || c == '!' || c == '?').count();
    let comma_count = TEXT.chars().filter(|&c| c == ',').count();
    println!("sentences={} commas={} chars={}", sentence_count, comma_count, TEXT.chars().count());
    let vowels = TEXT.chars().filter(|c| "aeiouAEIOU".contains(*c)).count();
    println!("vowel ratio {:.3}", vowels as f64 / TEXT.chars().filter(|c| c.is_alphabetic()).count() as f64);
    let anagram_key = |w: &str| {
        let mut cs: Vec<char> = w.chars().collect();
        cs.sort_unstable();
        cs.into_iter().collect::<String>()
    };
    let mut groups: BTreeMap<String, Vec<&str>> = BTreeMap::new();
    for w in ["listen", "silent", "enlist", "google", "gooegl", "cat", "act", "tac", "dog"] {
        groups.entry(anagram_key(w)).or_default().push(w);
    }
    println!("{:?}", groups.values().filter(|g| g.len() > 1).collect::<Vec<_>>());
}
