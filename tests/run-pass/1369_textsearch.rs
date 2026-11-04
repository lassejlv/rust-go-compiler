// Text search workloads: substring find/rfind/contains/split/rsplit/matches/
// replace/match_indices/rmatch_indices over random strings from tiny
// alphabets (lots of partial matches), checked against a naive reference;
// periodic worst-case inputs ("aaaa…ab") that need a linear-time search;
// a grep-like line filter with context, and an n-gram index.
use std::collections::{BTreeMap, HashMap};

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn string(&mut self, alphabet: &[char], len: usize) -> String {
        (0..len).map(|_| alphabet[(self.next() % alphabet.len() as u64) as usize]).collect()
    }
}

fn naive_find(h: &str, n: &str) -> Option<usize> {
    (0..=h.len()).filter(|&i| h.is_char_boundary(i)).find(|&i| h[i..].starts_with(n))
}

fn naive_rfind(h: &str, n: &str) -> Option<usize> {
    (0..=h.len()).rev().filter(|&i| h.is_char_boundary(i)).find(|&i| h[i..].starts_with(n))
}

fn naive_match_indices(h: &str, n: &str) -> Vec<usize> {
    let mut out = vec![];
    let mut i = 0;
    while i + n.len() <= h.len() {
        if h.is_char_boundary(i) && h[i..].starts_with(n) {
            out.push(i);
            i += n.len().max(1);
        } else {
            i += 1;
        }
    }
    out
}

fn grep<'a>(text: &'a str, needle: &str, context: usize) -> Vec<(usize, &'a str, bool)> {
    let lines: Vec<&str> = text.lines().collect();
    let hits: Vec<usize> = lines.iter().enumerate().filter(|(_, l)| l.contains(needle)).map(|(i, _)| i).collect();
    let mut keep = BTreeMap::new();
    for &h in &hits {
        let lo = h.saturating_sub(context);
        let hi = (h + context).min(lines.len() - 1);
        for i in lo..=hi {
            let e = keep.entry(i).or_insert(false);
            *e |= i == h;
        }
    }
    keep.into_iter().map(|(i, is_hit)| (i + 1, lines[i], is_hit)).collect()
}

fn main() {
    let mut r = Rng(0xDEADBEEF);
    let alphabets: [&[char]; 4] = [&['a', 'b'], &['a', 'b', 'c'], &['x', 'é', 'y'], &['a']];
    let mut checks = 0;
    let mut found = 0;
    for round in 0..400 {
        let al = alphabets[round % alphabets.len()];
        let hl = (r.next() % 300) as usize;
        let h = r.string(al, hl);
        let nl = 1 + (r.next() % 12) as usize;
        let n = r.string(al, nl);
        let f = h.find(n.as_str());
        assert_eq!(f, naive_find(&h, &n), "find {:?} in {:?}", n, h);
        assert_eq!(h.rfind(n.as_str()), naive_rfind(&h, &n), "rfind {:?} in {:?}", n, h);
        assert_eq!(h.contains(n.as_str()), f.is_some());
        let mi: Vec<usize> = h.match_indices(n.as_str()).map(|(i, _)| i).collect();
        assert_eq!(mi, naive_match_indices(&h, &n), "match_indices {:?} in {:?}", n, h);
        let parts: Vec<&str> = h.split(n.as_str()).collect();
        assert_eq!(parts.join(n.as_str()), h);
        assert_eq!(parts.len(), mi.len() + 1);
        let rparts: Vec<&str> = h.rsplit(n.as_str()).collect();
        assert_eq!(rparts.len(), h.rmatch_indices(n.as_str()).count() + 1);
        let rep = h.replace(n.as_str(), "#");
        assert_eq!(rep.matches('#').count(), mi.len());
        if f.is_some() {
            found += 1;
        }
        checks += 1;
    }
    println!("random checks {} (found {})", checks, found);

    // worst cases for naive search
    for &(hl, nl) in &[(200_000usize, 2_000usize), (100_000, 50_000), (300_000, 64)] {
        let hay = "a".repeat(hl) + "b" + &"a".repeat(nl);
        let needle = "a".repeat(nl) + "b";
        let rneedle = "b".to_string() + &"a".repeat(nl);
        let f = hay.find(needle.as_str());
        let rf = hay.rfind(rneedle.as_str());
        let missing = hay.find(&("a".repeat(nl) + "c"));
        let count = hay.matches(needle.as_str()).count();
        let pieces = hay.split(needle.as_str()).map(str::len).collect::<Vec<_>>();
        println!("periodic {}x{}: find {:?} rfind {:?} missing {:?} count {} pieces {:?}", hl, nl, f, rf, missing, count, pieces);
    }
    let fib = {
        let (mut a, mut b) = (String::from("a"), String::from("ab"));
        for _ in 0..20 {
            let c = b.clone() + &a;
            a = b;
            b = c;
        }
        b
    };
    let pat = &fib[..987];
    let occ: Vec<usize> = fib.match_indices(pat).map(|(i, _)| i).take(8).collect();
    println!("fibonacci word len {} occurrences of prefix {}: {} first {:?} last {:?}", fib.len(), pat.len(), fib.matches(pat).count(), occ, fib.rmatch_indices(pat).next().map(|(i, _)| i));

    let text = "alpha\nbeta gamma\ngamma ray\ndelta\nepsilon gamma\nzeta\neta\ntheta gamma\n";
    for (ln, line, hit) in grep(text, "gamma", 1) {
        println!("{}{:>3}: {}", if hit { '*' } else { ' ' }, ln, line);
    }

    // trigram index over generated words
    let words: Vec<String> = (0..2000).map(|_| {
        let l = 4 + (r.next() % 6) as usize;
        r.string(&['a', 'n', 't', 'r', 'e', 's'], l)
    }).collect();
    let mut index: HashMap<&str, Vec<usize>> = HashMap::new();
    for (i, w) in words.iter().enumerate() {
        for j in 0..w.len().saturating_sub(2) {
            let tri = &w[j..j + 3];
            let e = index.entry(tri).or_default();
            if e.last() != Some(&i) {
                e.push(i);
            }
        }
    }
    let query = "ante";
    let cands: Vec<usize> = {
        let lists: Vec<&Vec<usize>> = (0..query.len() - 2).filter_map(|j| index.get(&query[j..j + 3])).collect();
        let mut c = lists.iter().map(|l| l.len()).min().map(|_| lists[0].clone()).unwrap_or_default();
        for l in &lists[1..] {
            c.retain(|x| l.binary_search(x).is_ok());
        }
        c
    };
    let verified: Vec<&String> = cands.iter().map(|&i| &words[i]).filter(|w| w.contains(query)).collect();
    let brute = words.iter().filter(|w| w.contains(query)).count();
    println!("trigrams {} candidates {} verified {} brute {} sample {:?}", index.len(), cands.len(), verified.len(), brute, verified.iter().take(4).collect::<Vec<_>>());
    let s = "one, two,, three ,four";
    println!("{:?}", s.split(", ").collect::<Vec<_>>());
    println!("{:?}", s.rsplitn(2, ',').collect::<Vec<_>>());
    println!("{:?}", s.split_terminator(',').map(str::trim).collect::<Vec<_>>());
    println!("{:?} {:?}", s.split_once(",,"), s.rsplit_once(" "));
    println!("{:?}", "abcabcabc".replacen("bc", "_", 2));
    println!("{:?}", "  xxhixx ".trim().trim_start_matches("xx").trim_end_matches('x'));
}
