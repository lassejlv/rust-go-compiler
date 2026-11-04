// CSV parsing + a tiny SQL-ish query engine built from closures and iterator
// adaptors: SELECT cols WHERE pred GROUP BY key ORDER BY ... LIMIT n.
use std::borrow::Cow;
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fmt;
use std::rc::Rc;

#[derive(Debug, Clone, PartialEq, PartialOrd)]
enum Cell {
    Null,
    Int(i64),
    Float(f64),
    Text(Rc<str>),
}

impl Cell {
    fn parse(s: &str) -> Cell {
        let t = s.trim();
        if t.is_empty() || t.eq_ignore_ascii_case("null") {
            Cell::Null
        } else if let Ok(i) = t.parse::<i64>() {
            Cell::Int(i)
        } else if let Ok(f) = t.parse::<f64>() {
            Cell::Float(f)
        } else {
            Cell::Text(Rc::from(t))
        }
    }
    fn as_f64(&self) -> Option<f64> {
        match *self {
            Cell::Int(i) => Some(i as f64),
            Cell::Float(f) => Some(f),
            _ => None,
        }
    }
    fn text(&self) -> Cow<'_, str> {
        match self {
            Cell::Text(s) => Cow::Borrowed(s),
            Cell::Null => Cow::Borrowed(""),
            Cell::Int(i) => Cow::Owned(i.to_string()),
            Cell::Float(f) => Cow::Owned(format!("{:.2}", f)),
        }
    }
    fn total_cmp(&self, o: &Cell) -> Ordering {
        fn rank(c: &Cell) -> u8 {
            match c {
                Cell::Null => 0,
                Cell::Int(_) | Cell::Float(_) => 1,
                Cell::Text(_) => 2,
            }
        }
        match (self.as_f64(), o.as_f64()) {
            (Some(a), Some(b)) => a.total_cmp(&b),
            _ => rank(self).cmp(&rank(o)).then_with(|| self.text().cmp(&o.text())),
        }
    }
}

impl fmt::Display for Cell {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Cell::Null => f.pad("NULL"),
            Cell::Int(i) => f.pad(&i.to_string()),
            Cell::Float(x) => f.pad(&format!("{:.2}", x)),
            Cell::Text(s) => f.pad(s),
        }
    }
}

#[derive(Debug)]
enum CsvError {
    Ragged { line: usize, expected: usize, got: usize },
    UnclosedQuote(usize),
}

fn split_csv_line(line: &str, lineno: usize) -> Result<Vec<String>, CsvError> {
    let mut fields = Vec::new();
    let mut cur = String::new();
    let mut in_q = false;
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        match (c, in_q) {
            ('"', false) if cur.is_empty() => in_q = true,
            ('"', true) => {
                if chars.next_if_eq(&'"').is_some() {
                    cur.push('"');
                } else {
                    in_q = false;
                }
            }
            (',', false) => fields.push(std::mem::take(&mut cur)),
            (c, _) => cur.push(c),
        }
    }
    if in_q {
        return Err(CsvError::UnclosedQuote(lineno));
    }
    fields.push(cur);
    Ok(fields)
}

struct Table {
    cols: Vec<String>,
    rows: Vec<Vec<Cell>>,
}

impl Table {
    fn from_csv(src: &str) -> Result<Table, CsvError> {
        let mut lines = src.lines().enumerate().filter(|(_, l)| !l.trim().is_empty() && !l.starts_with('#'));
        let (hl, header) = lines.next().expect("header");
        let cols = split_csv_line(header, hl + 1)?.into_iter().map(|s| s.trim().to_lowercase()).collect::<Vec<_>>();
        let mut rows = Vec::new();
        for (i, l) in lines {
            let f = split_csv_line(l, i + 1)?;
            if f.len() != cols.len() {
                return Err(CsvError::Ragged { line: i + 1, expected: cols.len(), got: f.len() });
            }
            rows.push(f.iter().map(|s| Cell::parse(s)).collect());
        }
        Ok(Table { cols, rows })
    }
    fn col(&self, name: &str) -> Option<usize> {
        self.cols.iter().position(|c| c == name)
    }
}

type Pred = Box<dyn Fn(&[Cell]) -> bool>;
type Proj = Box<dyn Fn(&[Cell]) -> Cell>;

#[derive(Clone, Copy, Debug)]
enum Agg {
    Count,
    Sum,
    Avg,
    Min,
    Max,
}

struct Query<'t> {
    table: &'t Table,
    filters: Vec<Pred>,
    select: Vec<(String, Proj)>,
    group: Option<(String, usize, Vec<(String, Agg, usize)>)>,
    order: Vec<(usize, bool)>,
    limit: Option<usize>,
}

#[derive(Debug)]
enum QueryError {
    NoColumn(String),
}

impl<'t> Query<'t> {
    fn from(table: &'t Table) -> Self {
        Query { table, filters: vec![], select: vec![], group: None, order: vec![], limit: None }
    }
    fn idx(&self, c: &str) -> Result<usize, QueryError> {
        self.table.col(c).ok_or_else(|| QueryError::NoColumn(c.to_string()))
    }
    fn filter<F: Fn(&Cell) -> bool + 'static>(mut self, c: &str, f: F) -> Result<Self, QueryError> {
        let i = self.idx(c)?;
        self.filters.push(Box::new(move |row: &[Cell]| f(&row[i])));
        Ok(self)
    }
    fn col(mut self, c: &str) -> Result<Self, QueryError> {
        let i = self.idx(c)?;
        self.select.push((c.to_string(), Box::new(move |r: &[Cell]| r[i].clone())));
        Ok(self)
    }
    fn computed(mut self, name: &str, f: impl Fn(&[Cell]) -> Cell + 'static) -> Self {
        self.select.push((name.to_string(), Box::new(f)));
        self
    }
    fn group_by(mut self, key: &str, aggs: &[(&str, Agg)]) -> Result<Self, QueryError> {
        let k = self.idx(key)?;
        let a = aggs.iter().map(|(c, a)| Ok((format!("{:?}({})", a, c).to_lowercase(), *a, self.idx(c)?))).collect::<Result<Vec<_>, QueryError>>()?;
        self.group = Some((key.to_string(), k, a));
        Ok(self)
    }
    fn order_by(mut self, out_col: usize, desc: bool) -> Self {
        self.order.push((out_col, desc));
        self
    }
    fn limit(mut self, n: usize) -> Self {
        self.limit = Some(n);
        self
    }
    fn run(&self) -> (Vec<String>, Vec<Vec<Cell>>) {
        let matching = self.table.rows.iter().filter(|r| self.filters.iter().all(|f| f(r)));
        let (header, mut rows): (Vec<String>, Vec<Vec<Cell>>) = match &self.group {
            None => (self.select.iter().map(|(n, _)| n.clone()).collect(), matching.map(|r| self.select.iter().map(|(_, p)| p(r)).collect()).collect()),
            Some((kname, k, aggs)) => {
                let mut groups: BTreeMap<String, Vec<&Vec<Cell>>> = BTreeMap::new();
                for r in matching {
                    groups.entry(r[*k].text().into_owned()).or_default().push(r);
                }
                let header = std::iter::once(kname.clone()).chain(aggs.iter().map(|(n, _, _)| n.clone())).collect();
                let rows = groups
                    .into_iter()
                    .map(|(key, rs)| {
                        let mut out = vec![Cell::Text(key.into())];
                        for (_, agg, c) in aggs {
                            let nums = rs.iter().filter_map(|r| r[*c].as_f64());
                            out.push(match agg {
                                Agg::Count => Cell::Int(rs.iter().filter(|r| r[*c] != Cell::Null).count() as i64),
                                Agg::Sum => Cell::Float(nums.sum()),
                                Agg::Avg => {
                                    let v: Vec<f64> = nums.collect();
                                    if v.is_empty() {
                                        Cell::Null
                                    } else {
                                        Cell::Float(v.iter().sum::<f64>() / v.len() as f64)
                                    }
                                }
                                Agg::Min => rs.iter().map(|r| r[*c].clone()).filter(|c| *c != Cell::Null).min_by(|a, b| a.total_cmp(b)).unwrap_or(Cell::Null),
                                Agg::Max => rs.iter().map(|r| r[*c].clone()).max_by(|a, b| a.total_cmp(b)).unwrap_or(Cell::Null),
                            });
                        }
                        out
                    })
                    .collect();
                (header, rows)
            }
        };
        if !self.order.is_empty() {
            rows.sort_by(|a, b| {
                self.order.iter().fold(Ordering::Equal, |acc, &(c, desc)| {
                    acc.then_with(|| {
                        let o = a[c].total_cmp(&b[c]);
                        if desc {
                            o.reverse()
                        } else {
                            o
                        }
                    })
                })
            });
        }
        if let Some(n) = self.limit {
            rows.truncate(n);
        }
        (header, rows)
    }
}

fn render(header: &[String], rows: &[Vec<Cell>]) -> String {
    let widths: Vec<usize> = (0..header.len()).map(|i| rows.iter().map(|r| r[i].to_string().chars().count()).chain(std::iter::once(header[i].len())).max().unwrap_or(0)).collect();
    let mut out = String::new();
    let sep: String = widths.iter().map(|w| "-".repeat(w + 2)).collect::<Vec<_>>().join("+");
    out += &format!("+{}+\n", sep);
    let hdr: Vec<String> = header.iter().zip(&widths).map(|(h, w)| format!(" {:^w$} ", h, w = w)).collect();
    out += &format!("|{}|\n", hdr.join("|"));
    out += &format!("+{}+\n", sep);
    for r in rows {
        let cells: Vec<String> = r
            .iter()
            .zip(widths.iter())
            .map(|(c, &w)| match c {
                Cell::Int(_) | Cell::Float(_) => format!(" {:>w$} ", c, w = w),
                _ => format!(" {:<w$} ", c, w = w),
            })
            .collect();
        out += &format!("|{}|\n", cells.join("|"));
    }
    out += &format!("+{}+", sep);
    out
}

const DATA: &str = r#"name,dept,salary,age,city,start
# employees
Alice,Engineering,120000,34,"Seattle, WA",2015
Bob,Engineering,95000,28,Portland,2019
"Carol ""CJ"" Jones",Sales,70000,45,"Seattle, WA",2008
Dan,Sales,,52,Boise,2001
Eve,Engineering,135000.50,41,San Francisco,2012
Frank,Marketing,65000,23,Portland,2022
Grace,Marketing,72000,37,Seattle,2016
Heidi,Engineering,88000,26,Boise,2021
Ivan,Sales,81000,31,Portland,2018
Judy,HR,60000,29,null,2020
Mallory,HR,58000.25,48,Seattle,2010
"#;

fn main() {
    let t = Table::from_csv(DATA).expect("csv");
    println!("{} cols {:?}, {} rows", t.cols.len(), t.cols, t.rows.len());

    let q = Query::from(&t)
        .filter("dept", |c| c.text() == "Engineering")
        .and_then(|q| q.col("name"))
        .and_then(|q| q.col("salary"))
        .and_then(|q| q.col("age"))
        .map(|q| q.order_by(1, true))
        .unwrap();
    let (h, r) = q.run();
    println!("{}", render(&h, &r));

    let q = Query::from(&t).group_by("dept", &[("salary", Agg::Count), ("salary", Agg::Sum), ("salary", Agg::Avg), ("age", Agg::Min), ("age", Agg::Max)]).unwrap().order_by(3, true);
    let (h, r) = q.run();
    println!("{}", render(&h, &r));

    let sal = t.col("salary").unwrap();
    let start = t.col("start").unwrap();
    let q = Query::from(&t)
        .filter("age", |c| c.as_f64().map_or(false, |a| a >= 30.0))
        .unwrap()
        .col("name")
        .unwrap()
        .computed("tenure", move |r| match r[start] {
            Cell::Int(y) => Cell::Int(2024 - y),
            _ => Cell::Null,
        })
        .computed("per_year", move |r| match (r[sal].as_f64(), &r[start]) {
            (Some(s), Cell::Int(y)) => Cell::Float(s / (2024 - y) as f64),
            _ => Cell::Null,
        })
        .order_by(1, false)
        .order_by(0, false)
        .limit(5);
    let (h, r) = q.run();
    println!("{}", render(&h, &r));

    let q = Query::from(&t).group_by("city", &[("name", Agg::Count), ("age", Agg::Avg)]).unwrap().order_by(1, true).order_by(0, false);
    let (h, r) = q.run();
    println!("{}", render(&h, &r));

    match Query::from(&t).col("bogus") {
        Ok(_) => println!("unexpected"),
        Err(e) => println!("error: {:?}", e),
    }
    for bad in ["a,b\n1,2,3", "a,b\n\"x,1", "a\n1"] {
        match Table::from_csv(bad) {
            Ok(t) => println!("ok {} rows {:?}", t.rows.len(), t.rows),
            Err(e) => println!("csv error: {:?}", e),
        }
    }

    // ad-hoc analytics with iterator adaptors
    let name = t.col("name").unwrap();
    let city = t.col("city").unwrap();
    let cities: BTreeSet<Cow<str>> = t.rows.iter().map(|r| r[city].text()).filter(|c| !c.is_empty()).collect();
    println!("cities: {:?}", cities);
    let initials: String = t.rows.iter().filter_map(|r| r[name].text().chars().next()).collect();
    println!("initials: {}", initials);
    let mut by_len: HashMap<usize, Vec<String>> = HashMap::new();
    for r in &t.rows {
        let n = r[name].text().into_owned();
        by_len.entry(n.len()).or_default().push(n);
    }
    let mut lens: Vec<_> = by_len.into_iter().collect();
    lens.sort_unstable_by(|a, b| b.0.cmp(&a.0));
    println!("{:?}", &lens[..3]);
    let salaries: Vec<f64> = t.rows.iter().filter_map(|r| r[sal].as_f64()).collect();
    let mut sorted = salaries.clone();
    sorted.sort_by(f64::total_cmp);
    let median = if sorted.len() % 2 == 0 { (sorted[sorted.len() / 2 - 1] + sorted[sorted.len() / 2]) / 2.0 } else { sorted[sorted.len() / 2] };
    let (lo, hi): (Vec<f64>, Vec<f64>) = salaries.iter().partition(|&&s| s < median);
    println!("median {:.2}, below {}, at/above {}", median, lo.len(), hi.len());
    let pairs: Vec<String> = sorted.windows(2).map(|w| format!("{:.0}", w[1] - w[0])).collect();
    println!("gaps: {}", pairs.join(" "));
    let pos = sorted.partition_point(|&s| s < 80000.0);
    println!("first >= 80k at {} ({:?}); binary_search 95000 -> {:?}", pos, sorted.get(pos), sorted.binary_search_by(|x| x.total_cmp(&95000.0)));
    let total_by_dept = t.rows.iter().fold(BTreeMap::new(), |mut m, r| {
        *m.entry(r[1].text()).or_insert(0.0) += r[sal].as_f64().unwrap_or(0.0);
        m
    });
    println!("{:?}", total_by_dept);
    let oldest = t.rows.iter().max_by(|a, b| a[3].total_cmp(&b[3])).map(|r| r[name].to_string());
    let youngest = t.rows.iter().min_by_key(|r| r[3].as_f64().map(|x| x as i64)).map(|r| format!("{:>10}|{:<8}|{:^9}|", r[name], r[3], r[1]));
    println!("{:?} {:?}", oldest, youngest);
    println!("{:?}", Cell::Int(3).partial_cmp(&Cell::Float(1.0)));
}
