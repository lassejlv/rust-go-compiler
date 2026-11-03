// unzip, partition, group-by via fold, run-length encoding, histogram
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq)]
struct Sale {
    region: &'static str,
    product: &'static str,
    qty: u32,
    price: f64,
}

fn rle(s: &str) -> Vec<(char, usize)> {
    s.chars().fold(Vec::new(), |mut acc: Vec<(char, usize)>, c| {
        match acc.last_mut() {
            Some((lc, n)) if *lc == c => *n += 1,
            _ => acc.push((c, 1)),
        }
        acc
    })
}

fn group_consecutive<T: PartialEq + Clone>(v: &[T]) -> Vec<Vec<T>> {
    let mut out: Vec<Vec<T>> = vec![];
    for x in v {
        if out.last().map_or(false, |g| g[0] == *x) {
            out.last_mut().unwrap().push(x.clone());
        } else {
            out.push(vec![x.clone()]);
        }
    }
    out
}

fn main() {
    let sales = vec![
        Sale { region: "north", product: "apple", qty: 10, price: 0.5 },
        Sale { region: "south", product: "pear", qty: 4, price: 0.75 },
        Sale { region: "north", product: "pear", qty: 7, price: 0.8 },
        Sale { region: "east", product: "apple", qty: 3, price: 0.55 },
        Sale { region: "south", product: "apple", qty: 12, price: 0.45 },
        Sale { region: "north", product: "apple", qty: 1, price: 0.6 },
    ];
    let by_region = sales.iter().fold(BTreeMap::new(), |mut m: BTreeMap<&str, Vec<&Sale>>, s| {
        m.entry(s.region).or_default().push(s);
        m
    });
    for (r, ss) in &by_region {
        let rev: f64 = ss.iter().map(|s| s.qty as f64 * s.price).sum();
        println!("{:<6} n={} qty={} revenue={:.2}", r, ss.len(), ss.iter().map(|s| s.qty).sum::<u32>(), rev);
    }
    let by_prod: BTreeMap<&str, (u32, f64)> = sales.iter().fold(BTreeMap::new(), |mut m, s| {
        let e = m.entry(s.product).or_insert((0, 0.0));
        e.0 += s.qty;
        e.1 = f64::max(e.1, s.price);
        m
    });
    println!("{:?}", by_prod);
    let (big, small): (Vec<&Sale>, Vec<&Sale>) = sales.iter().partition(|s| s.qty >= 7);
    println!("big={:?}", big.iter().map(|s| s.qty).collect::<Vec<_>>());
    println!("small={:?}", small.iter().map(|s| (s.region, s.product)).collect::<Vec<_>>());
    let (regions, qtys): (Vec<&str>, Vec<u32>) = sales.iter().map(|s| (s.region, s.qty)).unzip();
    println!("{:?} {:?}", regions, qtys);
    let ((a, b), c): ((Vec<i32>, Vec<i32>), Vec<i32>) = (1..=4).map(|x| ((x, x * x), x * x * x)).unzip();
    println!("{:?} {:?} {:?}", a, b, c);
    println!("{:?}", rle("aaabccddddde"));
    let encoded: String = rle("wwwwbbbw").iter().map(|(c, n)| format!("{}{}", n, c)).collect();
    println!("{}", encoded);
    println!("{:?}", group_consecutive(&[1, 1, 2, 3, 3, 3, 1]));
    println!("{:?}", group_consecutive(&["a", "a", "b"]));
    let text = "it was the best of times it was the worst of times";
    let mut hist: BTreeMap<usize, Vec<&str>> = BTreeMap::new();
    for w in text.split(' ') {
        let e = hist.entry(w.len()).or_default();
        if !e.contains(&w) {
            e.push(w);
        }
    }
    for (len, ws) in hist.iter().rev() {
        println!("{} {} {:?}", len, "*".repeat(ws.len()), ws);
    }
    let evens_odds = (1..=10).fold((vec![], vec![]), |(mut e, mut o), x| {
        if x % 2 == 0 { e.push(x) } else { o.push(x) }
        (e, o)
    });
    println!("{:?}", evens_odds);
    let (oks, errs): (Vec<_>, Vec<_>) = ["1", "x", "3", "", "5"].iter().map(|s| s.parse::<u8>()).partition(Result::is_ok);
    let oks: Vec<u8> = oks.into_iter().map(Result::unwrap).collect();
    let errs: Vec<String> = errs.into_iter().map(|e| e.unwrap_err().to_string()).collect();
    println!("{:?} {:?}", oks, errs);
    let chunks: Vec<String> = sales.chunks(4).map(|c| c.iter().map(|s| s.product).collect::<Vec<_>>().join("/")).collect();
    println!("{:?}", chunks);
    let mut sorted = sales.clone();
    sorted.sort_by(|a, b| a.region.cmp(b.region).then(b.qty.cmp(&a.qty)));
    println!("{:?}", sorted.iter().map(|s| format!("{}:{}", s.region, s.qty)).collect::<Vec<_>>());
    println!("{}", sales.iter().position(|s| s == &sorted[0]).unwrap());
}
