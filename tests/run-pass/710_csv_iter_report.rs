// CSV parsing into typed records through iterator adapters, error collection,
// group-by aggregation with BTreeMap, and a formatted report.
use std::collections::BTreeMap;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Region { North, South, East, West }

impl FromStr for Region {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, String> {
        match s.to_ascii_lowercase().as_str() {
            "north" | "n" => Ok(Region::North),
            "south" | "s" => Ok(Region::South),
            "east" | "e" => Ok(Region::East),
            "west" | "w" => Ok(Region::West),
            other => Err(format!("unknown region '{}'", other)),
        }
    }
}

#[derive(Debug, Clone)]
struct Sale { day: u32, region: Region, product: String, qty: u32, price_cents: u64 }

#[derive(Debug)]
enum RowError { Fields(usize, usize), Num(usize, &'static str, String), Region(usize, String) }

fn split_csv(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut quoted = false;
    let mut it = line.chars().peekable();
    while let Some(c) = it.next() {
        match (c, quoted) {
            ('"', true) if it.peek() == Some(&'"') => { cur.push('"'); it.next(); }
            ('"', _) => quoted = !quoted,
            (',', false) => out.push(std::mem::take(&mut cur)),
            (c, _) => cur.push(c),
        }
    }
    out.push(cur);
    out
}

fn parse_price(s: &str) -> Option<u64> {
    let s = s.trim().trim_start_matches('$');
    let (whole, frac) = s.split_once('.').unwrap_or((s, "0"));
    let frac = format!("{:0<2}", frac);
    if frac.len() != 2 { return None; }
    Some(whole.parse::<u64>().ok()? * 100 + frac.parse::<u64>().ok()?)
}

fn parse_row(lineno: usize, line: &str) -> Result<Sale, RowError> {
    let f = split_csv(line);
    if f.len() != 5 { return Err(RowError::Fields(lineno, f.len())); }
    let day = f[0].trim().parse().map_err(|e: std::num::ParseIntError| RowError::Num(lineno, "day", e.to_string()))?;
    let region = f[1].trim().parse().map_err(|e| RowError::Region(lineno, e))?;
    let qty = f[3].trim().parse().map_err(|e: std::num::ParseIntError| RowError::Num(lineno, "qty", e.to_string()))?;
    let price_cents = parse_price(&f[4]).ok_or_else(|| RowError::Num(lineno, "price", f[4].clone()))?;
    Ok(Sale { day, region, product: f[2].trim().to_string(), qty, price_cents })
}

fn money(c: u64) -> String {
    let whole = (c / 100).to_string();
    let mut grouped = String::new();
    for (i, ch) in whole.chars().enumerate() {
        if i > 0 && (whole.len() - i) % 3 == 0 { grouped.push(','); }
        grouped.push(ch);
    }
    format!("${}.{:02}", grouped, c % 100)
}

const DATA: &str = "day,region,product,qty,price
1,North,Widget,10,$2.50
1,south,Gadget,3,19.99
2,E,\"Widget, Deluxe\",2,$120
2,West,Gizmo,7,4.5
3,north,Gadget,1,19.99
3,Nowhere,Widget,1,2.50
4,S,Widget,40,2.25
x,W,Gizmo,1,4.50
5,East,\"Thing \"\"Q\"\"\",5,1000.01
5,West,Gizmo
6,north,Widget,8,2.5.0
6,West,Widget,12,$2.40
7,East,Gadget,6,18.00
";

fn main() {
    let (rows, errors): (Vec<_>, Vec<_>) = DATA
        .lines()
        .enumerate()
        .skip(1)
        .filter(|(_, l)| !l.trim().is_empty())
        .map(|(i, l)| parse_row(i + 1, l))
        .partition(Result::is_ok);
    let sales: Vec<Sale> = rows.into_iter().map(Result::unwrap).collect();
    let errors: Vec<RowError> = errors.into_iter().filter_map(Result::err).collect();
    println!("{} good rows, {} errors", sales.len(), errors.len());
    for e in &errors {
        println!("  {:?}", e);
    }

    let mut by_region: BTreeMap<Region, (u32, u64)> = BTreeMap::new();
    for s in &sales {
        let e = by_region.entry(s.region).or_default();
        e.0 += s.qty;
        e.1 += s.qty as u64 * s.price_cents;
    }
    println!("{:<8}|{:>6}|{:>14}", "region", "qty", "revenue");
    println!("{}", "-".repeat(30));
    for (r, (q, rev)) in &by_region {
        println!("{:<8}|{:>6}|{:>14}", format!("{:?}", r), q, money(*rev));
    }
    let total: u64 = by_region.values().map(|v| v.1).sum();
    println!("{:<8}|{:>6}|{:>14}", "TOTAL", by_region.values().map(|v| v.0).sum::<u32>(), money(total));

    let mut by_product: BTreeMap<&str, Vec<&Sale>> = BTreeMap::new();
    for s in &sales {
        by_product.entry(s.product.as_str()).or_default().push(s);
    }
    for (p, v) in &by_product {
        let days: Vec<u32> = v.iter().map(|s| s.day).collect();
        let avg = v.iter().map(|s| s.price_cents).sum::<u64>() as f64 / v.len() as f64 / 100.0;
        println!("{:<18} n={} days={:?} avg_price={:.3}", format!("{:?}", p), v.len(), days, avg);
    }
    let best_day = sales
        .iter()
        .fold(BTreeMap::new(), |mut m: BTreeMap<u32, u64>, s| { *m.entry(s.day).or_insert(0) += s.qty as u64 * s.price_cents; m })
        .into_iter()
        .max_by_key(|&(d, v)| (v, std::cmp::Reverse(d)))
        .unwrap();
    println!("best day: {} with {}", best_day.0, money(best_day.1));
    let running: Vec<String> = sales
        .iter()
        .scan(0u64, |acc, s| { *acc += s.qty as u64 * s.price_cents; Some(money(*acc)) })
        .collect();
    println!("running: {}", running.join(" "));
    println!("split: {:?}", split_csv(r#"a,"b,c","d""e",,"#));
    println!("prices: {:?}", ["1", "1.5", "$0.07", "1.234", "abc", ".5"].map(parse_price));
    println!("money: {} {} {}", money(0), money(123456789), money(100000));
}
