// CSV analytics pipeline over embedded data: RFC4180-ish parser with quotes,
// typed columns, group-by aggregation, percentiles, joins, pivot table output.
use std::collections::{BTreeMap, HashMap};
use std::fmt;
use std::str::FromStr;

const SALES: &str = "\
id,date,region,rep,product,units,unit_price,notes
1,2024-01-03,North,Alice,Widget,12,2.50,
2,2024-01-04,South,Bob,Gadget,3,10.00,\"rush, priority\"
3,2024-01-04,North,Carol,Widget,7,2.50,
4,2024-01-09,East,Dave,Gizmo,1,99.99,\"said \"\"maybe\"\" twice\"
5,2024-01-11,West,Erin,Widget,30,2.25,bulk
6,2024-01-15,South,Bob,Widget,4,2.50,
7,2024-02-01,North,Alice,Gadget,2,10.00,
8,2024-02-02,East,Dave,Gadget,9,9.50,
9,2024-02-02,West,Erin,Gizmo,2,95.00,\"multi
line note\"
10,2024-02-10,North,Carol,Gizmo,1,99.99,
11,2024-02-14,South,Frank,Widget,15,2.40,
12,2024-02-20,East,Dave,Widget,0,2.50,cancelled
13,2024-03-01,West,Erin,Gadget,6,10.00,
14,2024-03-03,North,Alice,Widget,25,2.30,
15,2024-03-05,South,Bob,Gizmo,3,97.50,
16,2024-03-09,East,Gina,Gadget,4,10.00,
17,2024-03-15,North,Carol,Gadget,5,10.00,
18,2024-03-22,West,Erin,Widget,bad,2.50,typo
19,2024-03-28,South,Frank,Gadget,8,9.75,
20,2024-03-30,East,Gina,Gizmo,2,99.99,
";

const REPS: &str = "rep,hired,quota
Alice,2019,800
Bob,2021,600
Carol,2018,700
Dave,2020,650
Erin,2022,500
Frank,2023,400
";

fn parse_csv(text: &str) -> Result<Vec<Vec<String>>, String> {
    let mut rows = Vec::new();
    let mut row = Vec::new();
    let mut field = String::new();
    let mut in_quotes = false;
    let mut chars = text.chars().peekable();
    let mut line = 1;
    while let Some(c) = chars.next() {
        if in_quotes {
            match c {
                '"' if chars.peek() == Some(&'"') => {
                    chars.next();
                    field.push('"');
                }
                '"' => in_quotes = false,
                '\n' => {
                    line += 1;
                    field.push('\n');
                }
                c => field.push(c),
            }
        } else {
            match c {
                '"' if field.is_empty() => in_quotes = true,
                '"' => return Err(format!("line {}: stray quote", line)),
                ',' => row.push(std::mem::take(&mut field)),
                '\n' => {
                    line += 1;
                    row.push(std::mem::take(&mut field));
                    rows.push(std::mem::take(&mut row));
                }
                '\r' => {}
                c => field.push(c),
            }
        }
    }
    if in_quotes {
        return Err("unterminated quoted field".to_string());
    }
    if !field.is_empty() || !row.is_empty() {
        row.push(field);
        rows.push(row);
    }
    Ok(rows)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct Date { y: u16, m: u8, d: u8 }

impl FromStr for Date {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split('-').collect();
        if parts.len() != 3 {
            return Err(format!("bad date {:?}", s));
        }
        let y = parts[0].parse().map_err(|e| format!("year: {}", e))?;
        let m = parts[1].parse().map_err(|e| format!("month: {}", e))?;
        let d = parts[2].parse().map_err(|e| format!("day: {}", e))?;
        Ok(Date { y, m, d })
    }
}

impl fmt::Display for Date {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:04}-{:02}-{:02}", self.y, self.m, self.d)
    }
}

#[derive(Debug, Clone)]
struct Sale {
    id: u32,
    date: Date,
    region: String,
    rep: String,
    product: String,
    units: u32,
    price: f64,
    notes: String,
}

impl Sale {
    fn revenue(&self) -> f64 { self.units as f64 * self.price }
}

#[derive(Debug)]
enum RowError { Columns(usize, usize), Field { row: usize, col: &'static str, msg: String } }

impl fmt::Display for RowError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            RowError::Columns(r, n) => write!(f, "row {}: expected 8 columns, got {}", r, n),
            RowError::Field { row, col, msg } => write!(f, "row {}: column {}: {}", row, col, msg),
        }
    }
}

fn field<T: FromStr>(row: usize, col: &'static str, s: &str) -> Result<T, RowError>
where
    T::Err: fmt::Display,
{
    s.trim().parse::<T>().map_err(|e| RowError::Field { row, col, msg: e.to_string() })
}

fn to_sale(i: usize, r: &[String]) -> Result<Sale, RowError> {
    if r.len() != 8 {
        return Err(RowError::Columns(i, r.len()));
    }
    Ok(Sale {
        id: field(i, "id", &r[0])?,
        date: field(i, "date", &r[1])?,
        region: r[2].clone(),
        rep: r[3].clone(),
        product: r[4].clone(),
        units: field(i, "units", &r[5])?,
        price: field(i, "unit_price", &r[6])?,
        notes: r[7].clone(),
    })
}

#[derive(Default, Debug, Clone)]
struct Agg { n: usize, units: u32, revenue: f64, min: Option<f64>, max: Option<f64> }

impl Agg {
    fn add(&mut self, s: &Sale) {
        let r = s.revenue();
        self.n += 1;
        self.units += s.units;
        self.revenue += r;
        self.min = Some(self.min.map_or(r, |m| m.min(r)));
        self.max = Some(self.max.map_or(r, |m| m.max(r)));
    }
}

fn group_by<K: Ord + Clone>(sales: &[Sale], key: impl Fn(&Sale) -> K) -> BTreeMap<K, Agg> {
    let mut m: BTreeMap<K, Agg> = BTreeMap::new();
    for s in sales {
        m.entry(key(s)).or_default().add(s);
    }
    m
}

fn percentile(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return f64::NAN;
    }
    let rank = p / 100.0 * (sorted.len() - 1) as f64;
    let lo = rank.floor() as usize;
    let hi = rank.ceil() as usize;
    sorted[lo] + (sorted[hi] - sorted[lo]) * (rank - lo as f64)
}

fn main() {
    let rows = parse_csv(SALES).expect("csv");
    let header = &rows[0];
    println!("columns: {}", header.join(" | "));
    println!("raw rows: {}", rows.len() - 1);
    let mut sales = Vec::new();
    for (i, r) in rows.iter().enumerate().skip(1) {
        match to_sale(i, r) {
            Ok(s) => sales.push(s),
            Err(e) => println!("skip: {}", e),
        }
    }
    let with_notes: Vec<(u32, &str)> = sales.iter().filter(|s| !s.notes.is_empty()).map(|s| (s.id, s.notes.as_str())).collect();
    println!("notes: {:?}", with_notes);

    let total: f64 = sales.iter().map(Sale::revenue).sum();
    println!("valid sales: {}, total revenue {:.2}", sales.len(), total);

    println!("\nBy region:");
    println!("{:<8}{:>4}{:>7}{:>11}{:>9}{:>9}{:>7}", "region", "n", "units", "revenue", "min", "max", "share");
    for (k, a) in group_by(&sales, |s| s.region.clone()) {
        println!(
            "{:<8}{:>4}{:>7}{:>11.2}{:>9.2}{:>9.2}{:>6.1}%",
            k, a.n, a.units, a.revenue, a.min.unwrap(), a.max.unwrap(), a.revenue / total * 100.0
        );
    }

    println!("\nMonthly by product (pivot):");
    let months: Vec<u8> = {
        let mut m: Vec<u8> = sales.iter().map(|s| s.date.m).collect();
        m.sort_unstable();
        m.dedup();
        m
    };
    let pivot = group_by(&sales, |s| (s.product.clone(), s.date.m));
    let products: Vec<String> = {
        let mut p: Vec<String> = pivot.keys().map(|k| k.0.clone()).collect();
        p.dedup();
        p
    };
    print!("{:<8}", "product");
    for m in &months {
        print!("{:>10}", format!("2024-{:02}", m));
    }
    println!("{:>10}", "total");
    for p in &products {
        print!("{:<8}", p);
        let mut row_total = 0.0;
        for m in &months {
            let v = pivot.get(&(p.clone(), *m)).map_or(0.0, |a| a.revenue);
            row_total += v;
            if v == 0.0 {
                print!("{:>10}", "-");
            } else {
                print!("{:>10.2}", v);
            }
        }
        println!("{:>10.2}", row_total);
    }

    println!("\nRep performance vs quota:");
    let reps = parse_csv(REPS).unwrap();
    let quotas: HashMap<&str, (u16, f64)> = reps[1..].iter().map(|r| (r[0].as_str(), (r[1].parse().unwrap(), r[2].parse().unwrap()))).collect();
    let by_rep = group_by(&sales, |s| s.rep.clone());
    let mut perf: Vec<(String, f64, Option<f64>, Option<u16>)> = by_rep
        .iter()
        .map(|(rep, a)| {
            let q = quotas.get(rep.as_str());
            (rep.clone(), a.revenue, q.map(|q| a.revenue / q.1 * 100.0), q.map(|q| q.0))
        })
        .collect();
    perf.sort_by(|a, b| b.2.unwrap_or(-1.0).partial_cmp(&a.2.unwrap_or(-1.0)).unwrap().then_with(|| a.0.cmp(&b.0)));
    for (rep, rev, pct, hired) in &perf {
        let bar_len = pct.map_or(0, |p| (p / 10.0).round() as usize);
        println!(
            "{:<6} {:>8.2} {:>8} {:>6} {}",
            rep,
            rev,
            pct.map_or("n/a".to_string(), |p| format!("{:.1}%", p)),
            hired.map_or("?".to_string(), |h| h.to_string()),
            "=".repeat(bar_len.min(40))
        );
    }
    let unknown: Vec<&String> = by_rep.keys().filter(|r| !quotas.contains_key(r.as_str())).collect();
    let idle: Vec<&&str> = {
        let mut v: Vec<&&str> = quotas.keys().filter(|r| !by_rep.contains_key(**r)).collect();
        v.sort();
        v
    };
    println!("reps without quota: {:?}; reps without sales: {:?}", unknown, idle);

    println!("\nDistribution of sale revenue:");
    let mut revs: Vec<f64> = sales.iter().map(Sale::revenue).filter(|&r| r > 0.0).collect();
    revs.sort_by(|a, b| a.partial_cmp(b).unwrap());
    for p in [0.0, 10.0, 25.0, 50.0, 75.0, 90.0, 100.0] {
        println!("  p{:<3} = {:>8.3}", p, percentile(&revs, p));
    }
    let mean = revs.iter().sum::<f64>() / revs.len() as f64;
    let var = revs.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / (revs.len() - 1) as f64;
    println!("  mean = {:.4}, sd = {:.4}, n = {}", mean, var.sqrt(), revs.len());

    println!("\nRunning total by date:");
    let mut by_date: BTreeMap<Date, f64> = BTreeMap::new();
    for s in &sales {
        *by_date.entry(s.date).or_insert(0.0) += s.revenue();
    }
    let mut running = 0.0;
    let mut best_day = (Date { y: 0, m: 0, d: 0 }, f64::MIN);
    for (d, v) in &by_date {
        running += v;
        if *v > best_day.1 {
            best_day = (*d, *v);
        }
        if d.d <= 4 || *v > 100.0 {
            println!("  {} {:>8.2} {:>9.2}", d, v, running);
        }
    }
    println!("best day: {} ({:.2})", best_day.0, best_day.1);
    let range = by_date.range(Date { y: 2024, m: 2, d: 1 }..Date { y: 2024, m: 3, d: 1 });
    let feb: f64 = range.map(|(_, v)| v).sum();
    println!("February total: {:.2}", feb);

    // serialization back to CSV with quoting
    println!("\nRe-exported (top 3 by revenue):");
    let mut top = sales.clone();
    top.sort_by(|a, b| b.revenue().partial_cmp(&a.revenue()).unwrap().then(a.id.cmp(&b.id)));
    let quote = |s: &str| if s.contains(|c| c == ',' || c == '"' || c == '\n') { format!("\"{}\"", s.replace('"', "\"\"")) } else { s.to_string() };
    for s in top.iter().take(3) {
        println!("{},{},{},{},{}", s.id, s.date, quote(&s.rep), quote(&s.notes), s.revenue());
    }
    println!("{:?}", parse_csv("a,\"b\nc\"").map(|r| r.len()));
    println!("{:?}", parse_csv("a,b\"c\n"));
    println!("{:?}", parse_csv("\"open"));
    println!("{:?}", parse_csv("x,,\n,,\n"));
}
