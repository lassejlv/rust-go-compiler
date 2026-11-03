use std::collections::{BTreeMap, HashMap, HashSet};

struct Item { name: String, tags: Vec<String>, n: i32 }

fn count_closure() {
    let v = vec![String::from("a"), String::from("bb")];
    let mut count = 0;
    let names: Vec<&str> = v.iter().map(|s| { count += 1; s.as_str() }).collect();
    println!("{} {:?}", count, names);
    let mut seen = 0;
    let firsts: Vec<&String> = v.iter().filter(|s| { seen += 1; s.len() > 1 }).collect();
    seen += 1;
    println!("{} {:?}", seen, firsts);
    let mut calls = 0;
    let m = v.iter().map(|s| { calls += 1; s.len() }).max();
    calls += 1;
    println!("{:?} {}", m, calls);
}

fn chains() {
    let a = vec![1, 2, 3];
    let mut b = vec![4, 5];
    let total: i32 = a.iter().chain(b.iter()).sum();
    b.push(total);
    let pairs: Vec<(usize, &i32)> = a.iter().enumerate().collect();
    b.push(pairs.len() as i32);
    let z: Vec<(&i32, &i32)> = a.iter().zip(a.iter().rev()).collect();
    b.push(1);
    println!("{:?} {:?} {:?}", pairs, z, b);
    let words = String::from("x y z");
    let mut other = String::new();
    let ws: Vec<&str> = words.split(' ').filter(|w| !w.is_empty()).collect();
    other.push_str("q");
    let fm: Vec<&str> = vec![words.as_str()].into_iter().flat_map(|s| s.split(' ')).collect();
    other.push_str("r");
    println!("{:?} {:?} {}", ws, fm, other);
}

fn options() {
    let mut v = vec![String::from("a")];
    let n = v.first().map(|s| s.len());
    v.push(String::from("b"));
    let l = v.last().map(|s| s.as_str()).unwrap_or("").len();
    v.push(String::from("c"));
    let o: Option<String> = v.get(0).cloned();
    v.clear();
    let s = String::from("hello");
    let mut t = String::new();
    let r: Option<&str> = Some(s.as_str()).and_then(|x| x.get(0..2));
    t.push('a');
    let d = Some(String::from("q"));
    let dd: Option<&str> = d.as_deref();
    t.push('b');
    println!("{:?} {} {:?} {:?} {:?} {}", n, l, o, r, dd, t);
}

fn maps() {
    let k1 = String::from("k1");
    let mut val = String::from("v");
    let mut m: HashMap<&str, i32> = HashMap::new();
    m.insert(k1.as_str(), 1);
    *m.entry("x").or_insert(0) += 1;
    val.push('!');
    let got = m.get("k1").copied();
    let keys: Vec<&&str> = m.keys().collect();
    val.push('?');
    let mut bt: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    bt.entry(val.clone()).or_default().push(1);
    let lens: Vec<usize> = bt.values().map(|v| v.len()).collect();
    bt.insert(String::from("z"), vec![]);
    let hs: HashSet<&str> = k1.split('1').collect();
    val.push('.');
    println!("{:?} {} {:?} {:?} {}", got, keys.len(), lens, hs.len(), val);
}

fn structs() {
    let mut items = vec![Item { name: "a".into(), tags: vec!["t".into()], n: 1 }];
    let names: Vec<&String> = items.iter().map(|i| &i.name).collect();
    let total: i32 = items.iter().map(|i| i.n).sum();
    println!("{:?} {}", names, total);
    items.push(Item { name: "b".into(), tags: vec![], n: 2 });
    let tag_count: usize = items.iter().map(|i| i.tags.len()).sum();
    items[0].n += 1;
    for it in items.iter_mut() {
        it.n *= 2;
    }
    let by_name: HashMap<&str, &Item> = items.iter().map(|i| (i.name.as_str(), i)).collect();
    println!("{} {}", tag_count, by_name.len());
    items.retain(|i| i.n > 2);
    items.sort_by_key(|i| i.n);
    items.sort_by(|a, b| a.name.cmp(&b.name));
    println!("{}", items.len());
}

fn longest<'a>(x: &'a str, y: &'a str) -> &'a str { if x.len() > y.len() { x } else { y } }
fn pick<T>(v: &[T], i: usize) -> &T { &v[i] }
fn split_first_word(s: &str) -> (&str, &str) { match s.find(' ') { Some(i) => (&s[..i], &s[i + 1..]), None => (s, "") } }

fn sigs() {
    let a = String::from("abc");
    let mut b = String::from("de");
    let l = longest(&a, "zz");
    b.push('f');
    let p = pick(&[&a, &a], 0).len();
    b.push('g');
    let (x, y) = split_first_word(&a);
    b.push('h');
    let s: String = b.clone();
    let bb: &str = &s;
    let up = bb.to_uppercase();
    b.push_str(&up);
    println!("{} {} {} {} {}", l, p, x, y, b);
}

fn vec_of_refs() {
    let mut owner = vec![1, 2, 3];
    let mut refs: Vec<&i32> = Vec::new();
    {
        let other = vec![9];
        let tmp: Vec<&i32> = other.iter().collect();
        println!("{:?}", tmp);
    }
    refs.extend(owner.iter());
    println!("{:?}", refs);
    owner.push(4);
    let mut nums = vec![3, 1, 2];
    let mx = *nums.iter().max().unwrap();
    nums.push(mx);
    let pos = nums.iter().position(|&x| x == 2);
    nums.push(pos.unwrap() as i32);
    let mut grid = vec![vec![0; 3]; 3];
    let w = grid[0].len();
    grid[1][2] = w;
    let row: Vec<usize> = grid[1].iter().map(|x| x + 1).collect();
    grid[0][0] = row[2];
    println!("{:?} {:?} {:?}", owner, nums, grid);
}

fn main_fp() {
    count_closure();
    chains();
    options();
    maps();
    structs();
    sigs();
    vec_of_refs();
}

mod slots {
    use std::borrow::Cow;
    #[derive(Debug, Clone, Copy)]
    enum E<'a, 'b> { A(&'a str), B(&'b str), C { x: &'a str, y: &'b str } }
    #[derive(Debug, Clone, Copy)]
    struct T<'a, 'b> { x: &'a str, y: &'b str, n: i32 }
    fn first<'a, 'b>(e: E<'a, 'b>) -> Option<&'a str> {
        match e { E::A(s) => Some(s), E::C { x, .. } => Some(x), E::B(_) => None }
    }
    fn owned<'a>(v: Vec<Cow<'a, str>>) -> Vec<Cow<'static, str>> {
        v.into_iter().map(|p| Cow::Owned(p.into_owned())).collect()
    }
    fn kinds(v: &[String]) -> Vec<&'static str> {
        v.iter().map(|s| if s.len() > 1 { "long" } else { "short" }).collect()
    }
    pub fn main() {
        let mut a = String::from("a");
        let b = String::from("b");
        let e = E::A(&b);
        let e2 = E::B(&a);
        let e3 = E::C { y: &a, x: &b };
        let t = T { y: &a, x: &b, n: 1 };
        let t2 = T { n: 2, ..t };
        let x = match e3 { E::C { x, .. } => x, _ => "" };
        let tx = t2.x;
        let f = first(e3);
        println!("{:?} {} {}", e2, t.n, t2.y);
        a.push('!');
        println!("{:?} {} {} {:?} {}", e, x, tx, f, a);
        let s = String::from("x");
        let v = vec![Cow::Borrowed(s.as_str())];
        let o = owned(v);
        drop(s);
        println!("{:?}", o);
        let mut names = vec![String::from("ab"), String::from("c")];
        let k = kinds(&names);
        names.push(String::from("d"));
        let k2: Vec<&'static str> = names.iter().map(|s| if s.is_empty() { "e" } else { "n" }).collect();
        names.clear();
        println!("{:?} {:?} {:?}", k, k2, names);
    }
}

fn main() {
    main_fp();
    slots::main();
}
