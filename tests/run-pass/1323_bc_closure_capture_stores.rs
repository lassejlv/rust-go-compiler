// Closures storing into captured variables what other captures borrow
// (the stored loans flow into the outer variable): valid uses.
use std::collections::HashMap;

fn store_ref() {
    let s = String::from("x");
    let mut o = None;
    let mut f = || o = Some(&s);
    f();
    println!("{:?}", o);
}

fn push_words() {
    let mut count = 0;
    let mut v: Vec<&str> = Vec::new();
    let text = String::from("a b c");
    let mut f = || {
        for w in text.split(' ') {
            v.push(w);
            count += 1;
        }
    };
    f();
    println!("{:?} {}", v, count);
}

fn pick_best() {
    let words = vec!["a".to_string(), "bb".to_string()];
    let mut best: Option<&String> = None;
    let mut consider = |i: usize| {
        if best.map_or(true, |b| b.len() < words[i].len()) {
            best = Some(&words[i]);
        }
    };
    consider(0);
    consider(1);
    println!("{:?}", best);
}

fn copied_ref_capture() {
    let s = String::from("y");
    let mut o = None;
    {
        let r: &str = &s;
        let mut f = || o = Some(r);
        f();
    }
    println!("{:?}", o);
}

fn store_len_then_mutate() {
    let mut s = String::from("abc");
    let mut v: Vec<usize> = Vec::new();
    {
        let mut f = || v.push(s.len());
        f();
    }
    s.push('d');
    println!("{:?} {}", v, s);
}

fn push_str_from_local() {
    let mut total = String::new();
    {
        let t = String::from("zz");
        let mut f = || total.push_str(&t);
        f();
    }
    println!("{}", total);
}

fn fill_cache() {
    let src = vec![String::from("p"), String::from("q")];
    let mut cache: Vec<&str> = Vec::new();
    let mut fill = || {
        for x in &src {
            cache.push(x);
        }
        cache.len()
    };
    println!("{}", fill());
    println!("{:?}", cache);
}

fn entry_counts() {
    let keys = vec!["a", "b", "a"];
    let mut m: HashMap<&str, i32> = HashMap::new();
    keys.iter().for_each(|k| *m.entry(k).or_insert(0) += 1);
    let mut ks: Vec<_> = m.into_iter().collect();
    ks.sort();
    println!("{:?}", ks);
}

fn optional_slot() {
    let opt = Some(String::from("z"));
    let mut slot: Option<&str> = None;
    let mut set = || {
        if let Some(z) = &opt {
            slot = Some(z.as_str());
        }
    };
    set();
    println!("{:?}", slot);
}

fn returns_capture() {
    let s = String::from("cap");
    let r = &s;
    let f = move || r;
    let x = f();
    let v = vec![String::from("a"), String::from("b")];
    let g = |i: usize| v[i].as_str();
    let (a, b) = (g(0), g(1));
    println!("{} {} {}", x, a, b);
}

fn main() {
    store_ref();
    push_words();
    pick_best();
    copied_ref_capture();
    store_len_then_mutate();
    push_str_from_local();
    fill_cache();
    entry_counts();
    optional_slot();
    returns_capture();
}
