// More valid borrow patterns: callbacks through fn pointers / dyn Fn taking
// and returning references, storing call results that don't borrow the
// argument, reborrows passed to generic functions, iterators returned from
// methods while other fields are mutated, and get-or-insert caches.
use std::collections::HashMap;

type Extract = fn(&str) -> usize;
type Pick = for<'a> fn(&'a [String]) -> Option<&'a String>;

struct Engine { rules: Vec<(String, Extract)>, hits: Vec<usize>, picks: Vec<Pick>, cache: HashMap<String, Vec<usize>>, log: Vec<String> }

impl Engine {
    fn run(&mut self, input: &str) -> usize {
        let mut total = 0;
        for (name, f) in &self.rules {
            let n = f(input);
            self.log.push(format!("{}={}", name, n));
            total += n;
        }
        self.hits.push(total);
        total
    }
    fn run_dyn(&mut self, input: &str, extra: &dyn Fn(&str) -> String) -> String {
        let out = extra(input);
        self.log.push(out.clone());
        out
    }
    fn best<'a>(&self, items: &'a [String]) -> Vec<&'a String> {
        self.picks.iter().filter_map(|p| p(items)).collect()
    }
    fn lengths(&mut self, key: &str) -> &Vec<usize> {
        if !self.cache.contains_key(key) {
            let v: Vec<usize> = key.split('/').map(|s| s.len()).collect();
            self.cache.insert(key.to_string(), v);
        }
        &self.cache[key]
    }
    fn names(&self) -> impl Iterator<Item = &str> + '_ { self.rules.iter().map(|(n, _)| n.as_str()) }
}

fn count_vowels(s: &str) -> usize { s.chars().filter(|c| "aeiou".contains(*c)).count() }
fn longest(v: &[String]) -> Option<&String> { v.iter().max_by_key(|s| s.len()) }
fn first(v: &[String]) -> Option<&String> { v.first() }

fn apply<F: FnMut(&mut Vec<i32>)>(v: &mut Vec<i32>, mut f: F) { f(v); f(v); }
fn push_twice<T: Clone>(v: &mut Vec<T>, x: T) { v.push(x.clone()); v.push(x); }

fn main() {
    let mut e = Engine {
        rules: vec![("len".into(), str::len), ("vowels".into(), count_vowels), ("words".into(), |s| s.split_whitespace().count())],
        hits: vec![], picks: vec![longest, first], cache: HashMap::new(), log: vec![],
    };
    let line = String::from("the quick brown fox");
    let t1 = e.run(&line);
    let t2 = e.run("aeiou");
    println!("totals {} {} hits {:?}", t1, t2, e.hits);
    let suffix = String::from("!");
    let shout = |s: &str| format!("{}{}", s.to_uppercase(), suffix);
    let r = e.run_dyn(&line, &shout);
    println!("dyn {} log {:?}", r, e.log);
    let items: Vec<String> = ["pear", "watermelon", "fig"].iter().map(|s| s.to_string()).collect();
    let best = e.best(&items);
    e.log.clear();
    println!("best {:?}", best);
    let l1 = e.lengths("usr/local/bin").clone();
    let l2 = e.lengths("a/bb").len();
    let l3 = e.lengths("usr/local/bin");
    println!("cache {:?} {} {:?}", l1, l2, l3);
    let names: Vec<&str> = e.names().collect();
    println!("names {:?}", names);
    e.hits.push(names.len());
    println!("hits after {:?}", e.hits);

    let mut v = vec![1, 2];
    apply(&mut v, |w| { let n = w.len() as i32; w.push(n); });
    let r = &mut v;
    push_twice(r, 9);
    push_twice(&mut *r, 8);
    r.sort();
    println!("v {:?}", v);

    let mut words: HashMap<&str, Vec<&str>> = HashMap::new();
    let text = "apple avocado banana blueberry cherry";
    for w in text.split(' ') {
        let k = &w[..1];
        words.entry(k).or_default().push(w);
    }
    let mut keys: Vec<_> = words.keys().copied().collect();
    keys.sort();
    let summary: Vec<String> = keys.iter().map(|k| format!("{}:{}", k, words[k].join("+"))).collect();
    println!("{:?}", summary);
    if let Some(list) = words.get_mut("b") { list.retain(|w| w.len() > 6); }
    println!("b now {:?}", words["b"]);

    let mut state = (0u32, String::new());
    let mut step = |c: char| { state.0 += 1; state.1.push(c); state.0 };
    let counts: Vec<u32> = "xyz".chars().map(&mut step).collect();
    println!("{:?} {:?}", counts, state);

    let mut matrix = vec![vec![1, 2], vec![3, 4]];
    let total: i32 = matrix.iter().flatten().sum();
    for row in matrix.iter_mut() { for x in row.iter_mut() { *x *= total; } }
    let first_row_sum: i32 = matrix[0].iter().sum();
    matrix[1].push(first_row_sum);
    println!("{:?}", matrix);
}
