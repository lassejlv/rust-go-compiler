// A tiny relational engine: tables with typed columns, a logical plan tree
// (scan / filter / project / hash join / merge join / sort / limit /
// aggregate), execution as a pipeline of boxed iterators borrowing the
// catalog, an index (BTreeMap) used for range scans, EXPLAIN output, and
// cross-checks between join strategies.
use std::cmp::Ordering;
use std::collections::{BTreeMap, HashMap};
use std::fmt;
use std::rc::Rc;

#[derive(Debug, Clone, PartialEq, PartialOrd)]
enum Datum {
    Null,
    Int(i64),
    Real(f64),
    Str(Rc<str>),
}

impl Datum {
    fn cmp_total(&self, o: &Datum) -> Ordering {
        use Datum::*;
        match (self, o) {
            (Null, Null) => Ordering::Equal,
            (Null, _) => Ordering::Less,
            (_, Null) => Ordering::Greater,
            (Int(a), Int(b)) => a.cmp(b),
            (Real(a), Real(b)) => a.total_cmp(b),
            (Int(a), Real(b)) => (*a as f64).total_cmp(b),
            (Real(a), Int(b)) => a.total_cmp(&(*b as f64)),
            (Str(a), Str(b)) => a.cmp(b),
            (Str(_), _) => Ordering::Greater,
            (_, Str(_)) => Ordering::Less,
        }
    }
    fn as_f64(&self) -> Option<f64> {
        match self {
            Datum::Int(i) => Some(*i as f64),
            Datum::Real(r) => Some(*r),
            _ => None,
        }
    }
    fn key(&self) -> String {
        match self {
            Datum::Null => "\u{0}".into(),
            Datum::Int(i) => format!("i{}", i),
            Datum::Real(r) => format!("r{}", r),
            Datum::Str(s) => format!("s{}", s),
        }
    }
}

impl fmt::Display for Datum {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Datum::Null => f.pad("NULL"),
            Datum::Int(i) => f.pad(&i.to_string()),
            Datum::Real(r) => f.pad(&format!("{:.2}", r)),
            Datum::Str(s) => f.pad(s),
        }
    }
}

type Row = Vec<Datum>;

struct Table {
    name: String,
    cols: Vec<String>,
    rows: Vec<Row>,
    index: HashMap<usize, BTreeMap<String, Vec<usize>>>,
}

impl Table {
    fn new(name: &str, cols: &[&str]) -> Table {
        Table { name: name.into(), cols: cols.iter().map(|s| s.to_string()).collect(), rows: vec![], index: HashMap::new() }
    }
    fn insert(&mut self, r: Row) {
        assert_eq!(r.len(), self.cols.len(), "arity mismatch in {}", self.name);
        let id = self.rows.len();
        for (col, idx) in self.index.iter_mut() {
            idx.entry(r[*col].key()).or_default().push(id);
        }
        self.rows.push(r);
    }
    fn create_index(&mut self, col: &str) {
        let c = self.cols.iter().position(|x| x == col).unwrap();
        let mut idx: BTreeMap<String, Vec<usize>> = BTreeMap::new();
        for (i, r) in self.rows.iter().enumerate() {
            idx.entry(r[c].key()).or_default().push(i);
        }
        self.index.insert(c, idx);
    }
}

#[derive(Clone)]
enum Pred {
    Eq(usize, Datum),
    Lt(usize, Datum),
    Ge(usize, Datum),
    And(Box<Pred>, Box<Pred>),
    Or(Box<Pred>, Box<Pred>),
    Not(Box<Pred>),
    ColEq(usize, usize),
}

impl Pred {
    fn eval(&self, r: &Row) -> bool {
        match self {
            Pred::Eq(c, d) => r[*c].cmp_total(d) == Ordering::Equal,
            Pred::Lt(c, d) => r[*c].cmp_total(d) == Ordering::Less,
            Pred::Ge(c, d) => r[*c].cmp_total(d) != Ordering::Less,
            Pred::And(a, b) => a.eval(r) && b.eval(r),
            Pred::Or(a, b) => a.eval(r) || b.eval(r),
            Pred::Not(a) => !a.eval(r),
            Pred::ColEq(a, b) => r[*a] == r[*b],
        }
    }
    fn show(&self, cols: &[String]) -> String {
        match self {
            Pred::Eq(c, d) => format!("{} = {}", cols[*c], d),
            Pred::Lt(c, d) => format!("{} < {}", cols[*c], d),
            Pred::Ge(c, d) => format!("{} >= {}", cols[*c], d),
            Pred::And(a, b) => format!("({} AND {})", a.show(cols), b.show(cols)),
            Pred::Or(a, b) => format!("({} OR {})", a.show(cols), b.show(cols)),
            Pred::Not(a) => format!("NOT {}", a.show(cols)),
            Pred::ColEq(a, b) => format!("{} = {}", cols[*a], cols[*b]),
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum AggFn {
    Count,
    Sum,
    Avg,
    Max,
}

enum Plan {
    Scan(String),
    IndexEq { table: String, col: String, value: Datum },
    Filter(Box<Plan>, Pred),
    Project(Box<Plan>, Vec<usize>),
    HashJoin { left: Box<Plan>, right: Box<Plan>, lkey: usize, rkey: usize },
    MergeJoin { left: Box<Plan>, right: Box<Plan>, lkey: usize, rkey: usize },
    Sort(Box<Plan>, Vec<(usize, bool)>),
    Limit(Box<Plan>, usize),
    Aggregate { input: Box<Plan>, group: Vec<usize>, aggs: Vec<(AggFn, usize)> },
}

struct Db {
    tables: HashMap<String, Table>,
}

type RowIter<'a> = Box<dyn Iterator<Item = Row> + 'a>;

impl Db {
    fn columns(&self, p: &Plan) -> Vec<String> {
        match p {
            Plan::Scan(t) | Plan::IndexEq { table: t, .. } => self.tables[t].cols.iter().map(|c| format!("{}.{}", t, c)).collect(),
            Plan::Filter(i, _) | Plan::Sort(i, _) | Plan::Limit(i, _) => self.columns(i),
            Plan::Project(i, cols) => {
                let c = self.columns(i);
                cols.iter().map(|&x| c[x].clone()).collect()
            }
            Plan::HashJoin { left, right, .. } | Plan::MergeJoin { left, right, .. } => {
                let mut c = self.columns(left);
                c.extend(self.columns(right));
                c
            }
            Plan::Aggregate { input, group, aggs } => {
                let c = self.columns(input);
                group.iter().map(|&g| c[g].clone()).chain(aggs.iter().map(|(f, col)| format!("{:?}({})", f, c[*col]).to_lowercase())).collect()
            }
        }
    }

    fn explain(&self, p: &Plan, depth: usize, out: &mut Vec<String>) {
        let pad = "  ".repeat(depth);
        let line = match p {
            Plan::Scan(t) => format!("Scan {} ({} rows)", t, self.tables[t].rows.len()),
            Plan::IndexEq { table, col, value } => format!("IndexLookup {}.{} = {}", table, col, value),
            Plan::Filter(i, pr) => format!("Filter {}", pr.show(&self.columns(i))),
            Plan::Project(i, cols) => {
                let c = self.columns(i);
                format!("Project {}", cols.iter().map(|&x| c[x].as_str()).collect::<Vec<_>>().join(", "))
            }
            Plan::HashJoin { left, right, lkey, rkey } => format!("HashJoin {} = {}", self.columns(left)[*lkey], self.columns(right)[*rkey]),
            Plan::MergeJoin { left, right, lkey, rkey } => format!("MergeJoin {} = {}", self.columns(left)[*lkey], self.columns(right)[*rkey]),
            Plan::Sort(i, keys) => {
                let c = self.columns(i);
                format!("Sort {}", keys.iter().map(|(k, d)| format!("{}{}", c[*k], if *d { " DESC" } else { "" })).collect::<Vec<_>>().join(", "))
            }
            Plan::Limit(_, n) => format!("Limit {}", n),
            Plan::Aggregate { group, aggs, .. } => format!("Aggregate group={:?} aggs={:?}", group, aggs),
        };
        out.push(format!("{}{}", pad, line));
        match p {
            Plan::Scan(_) | Plan::IndexEq { .. } => {}
            Plan::Filter(i, _) | Plan::Project(i, _) | Plan::Sort(i, _) | Plan::Limit(i, _) | Plan::Aggregate { input: i, .. } => self.explain(i, depth + 1, out),
            Plan::HashJoin { left, right, .. } | Plan::MergeJoin { left, right, .. } => {
                self.explain(left, depth + 1, out);
                self.explain(right, depth + 1, out);
            }
        }
    }

    fn exec<'a>(&'a self, p: &'a Plan) -> RowIter<'a> {
        match p {
            Plan::Scan(t) => Box::new(self.tables[t].rows.iter().cloned()),
            Plan::IndexEq { table, col, value } => {
                let t = &self.tables[table];
                let c = t.cols.iter().position(|x| x == col).unwrap();
                let ids: Vec<usize> = match t.index.get(&c) {
                    Some(idx) => idx.get(&value.key()).cloned().unwrap_or_default(),
                    None => t.rows.iter().enumerate().filter(|(_, r)| r[c] == *value).map(|(i, _)| i).collect(),
                };
                Box::new(ids.into_iter().map(move |i| t.rows[i].clone()))
            }
            Plan::Filter(i, pred) => Box::new(self.exec(i).filter(move |r| pred.eval(r))),
            Plan::Project(i, cols) => Box::new(self.exec(i).map(move |r| cols.iter().map(|&c| r[c].clone()).collect())),
            Plan::HashJoin { left, right, lkey, rkey } => {
                let mut table: HashMap<String, Vec<Row>> = HashMap::new();
                for r in self.exec(right) {
                    table.entry(r[*rkey].key()).or_default().push(r);
                }
                Box::new(self.exec(left).flat_map(move |l| {
                    let matches = table.get(&l[*lkey].key()).cloned().unwrap_or_default();
                    matches.into_iter().map(move |r| {
                        let mut out = l.clone();
                        out.extend(r);
                        out
                    })
                }))
            }
            Plan::MergeJoin { left, right, lkey, rkey } => {
                let mut l: Vec<Row> = self.exec(left).collect();
                let mut r: Vec<Row> = self.exec(right).collect();
                l.sort_by(|a, b| a[*lkey].cmp_total(&b[*lkey]));
                r.sort_by(|a, b| a[*rkey].cmp_total(&b[*rkey]));
                let mut out = Vec::new();
                let (mut i, mut j) = (0, 0);
                while i < l.len() && j < r.len() {
                    match l[i][*lkey].cmp_total(&r[j][*rkey]) {
                        Ordering::Less => i += 1,
                        Ordering::Greater => j += 1,
                        Ordering::Equal => {
                            let j0 = j;
                            while j < r.len() && r[j][*rkey].cmp_total(&l[i][*lkey]) == Ordering::Equal {
                                let mut row = l[i].clone();
                                row.extend(r[j].iter().cloned());
                                out.push(row);
                                j += 1;
                            }
                            i += 1;
                            if i < l.len() && l[i][*lkey].cmp_total(&l[i - 1][*lkey]) == Ordering::Equal {
                                j = j0;
                            }
                        }
                    }
                }
                Box::new(out.into_iter())
            }
            Plan::Sort(i, keys) => {
                let mut rows: Vec<Row> = self.exec(i).collect();
                rows.sort_by(|a, b| keys.iter().map(|&(k, desc)| if desc { b[k].cmp_total(&a[k]) } else { a[k].cmp_total(&b[k]) }).find(|o| *o != Ordering::Equal).unwrap_or(Ordering::Equal));
                Box::new(rows.into_iter())
            }
            Plan::Limit(i, n) => Box::new(self.exec(i).take(*n)),
            Plan::Aggregate { input, group, aggs } => {
                let mut groups: BTreeMap<Vec<String>, (Row, Vec<(f64, i64, f64)>)> = BTreeMap::new();
                for r in self.exec(input) {
                    let key: Vec<String> = group.iter().map(|&g| r[g].key()).collect();
                    let entry = groups.entry(key).or_insert_with(|| (group.iter().map(|&g| r[g].clone()).collect(), vec![(0.0, 0, f64::NEG_INFINITY); aggs.len()]));
                    for (slot, (_, col)) in entry.1.iter_mut().zip(aggs) {
                        if let Some(x) = r[*col].as_f64() {
                            slot.0 += x;
                            slot.1 += 1;
                            slot.2 = slot.2.max(x);
                        } else if r[*col] != Datum::Null {
                            slot.1 += 1;
                        }
                    }
                }
                Box::new(groups.into_values().map(move |(mut keyrow, accs)| {
                    for ((f, _), (sum, n, max)) in aggs.iter().zip(accs) {
                        keyrow.push(match f {
                            AggFn::Count => Datum::Int(n),
                            AggFn::Sum => Datum::Real(sum),
                            AggFn::Avg if n == 0 => Datum::Null,
                            AggFn::Avg => Datum::Real(sum / n as f64),
                            AggFn::Max if n == 0 => Datum::Null,
                            AggFn::Max => Datum::Real(max),
                        });
                    }
                    keyrow
                }))
            }
        }
    }

    fn run(&self, title: &str, p: &Plan) -> Vec<Row> {
        let cols = self.columns(p);
        let rows: Vec<Row> = self.exec(p).collect();
        println!("== {} ==", title);
        let mut ex = vec![];
        self.explain(p, 1, &mut ex);
        for l in ex {
            println!("{}", l);
        }
        let widths: Vec<usize> = (0..cols.len()).map(|i| rows.iter().map(|r| r[i].to_string().len()).chain([cols[i].len()]).max().unwrap()).collect();
        println!("{}", cols.iter().zip(&widths).map(|(c, w)| format!("{:<w$}", c, w = w)).collect::<Vec<_>>().join(" | "));
        for r in rows.iter().take(8) {
            println!("{}", r.iter().zip(&widths).map(|(d, w)| match d {
                Datum::Str(_) | Datum::Null => format!("{:<w$}", d, w = w),
                _ => format!("{:>w$}", d, w = w),
            }).collect::<Vec<_>>().join(" | "));
        }
        if rows.len() > 8 {
            println!("... {} rows", rows.len());
        }
        rows
    }
}

fn s(x: &str) -> Datum {
    Datum::Str(Rc::from(x))
}

fn main() {
    let mut users = Table::new("users", &["id", "name", "city", "age"]);
    let cities = ["Oslo", "Lima", "Pune", "Kyiv", "Baku"];
    let names = ["ana", "bo", "cy", "dee", "eli", "fay", "gus", "hal", "ivy", "jo", "kai", "liv"];
    for (i, n) in names.iter().enumerate() {
        users.insert(vec![Datum::Int(i as i64 + 1), s(n), s(cities[i * 7 % 5]), if i % 5 == 3 { Datum::Null } else { Datum::Int(18 + (i as i64 * 13) % 50) }]);
    }
    users.create_index("city");
    let mut orders = Table::new("orders", &["oid", "uid", "amount", "item"]);
    let items = ["book", "pen", "lamp", "desk", "mug"];
    let mut x = 7u64;
    for oid in 0..60 {
        x = x.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let uid = (x >> 40) % 14; // some orders reference missing users
        let amount = ((x >> 20) % 10_000) as f64 / 100.0;
        orders.insert(vec![Datum::Int(oid), Datum::Int(uid as i64), Datum::Real(amount), s(items[(x >> 50) as usize % items.len()])]);
    }
    orders.create_index("uid");
    let db = Db { tables: [("users".to_string(), users), ("orders".to_string(), orders)].into_iter().collect() };

    db.run("adults in Lima or Oslo", &Plan::Sort(
        Box::new(Plan::Filter(
            Box::new(Plan::Scan("users".into())),
            Pred::And(Box::new(Pred::Ge(3, Datum::Int(30))), Box::new(Pred::Or(Box::new(Pred::Eq(2, s("Lima"))), Box::new(Pred::Eq(2, s("Oslo")))))),
        )),
        vec![(3, true), (1, false)],
    ));
    db.run("index lookup", &Plan::Project(Box::new(Plan::IndexEq { table: "users".into(), col: "city".into(), value: s("Pune") }), vec![1, 3]));
    let join = |hash: bool| -> Plan {
        let (left, right) = (Box::new(Plan::Scan("orders".into())), Box::new(Plan::Scan("users".into())));
        if hash {
            Plan::HashJoin { left, right, lkey: 1, rkey: 0 }
        } else {
            Plan::MergeJoin { left, right, lkey: 1, rkey: 0 }
        }
    };
    let h = db.run("hash join", &Plan::Limit(Box::new(Plan::Sort(Box::new(join(true)), vec![(2, true)])), 5));
    let mut hj: Vec<String> = db.exec(&join(true)).map(|r| format!("{:?}", r)).collect();
    let mut mj: Vec<String> = db.exec(&join(false)).map(|r| format!("{:?}", r)).collect();
    hj.sort();
    mj.sort();
    println!("hash join rows {} merge join rows {} identical {} top amount {}", hj.len(), mj.len(), hj == mj, h[0][2]);
    let agg = Plan::Sort(
        Box::new(Plan::Aggregate { input: Box::new(join(true)), group: vec![6], aggs: vec![(AggFn::Count, 0), (AggFn::Sum, 2), (AggFn::Avg, 2), (AggFn::Max, 7)] }),
        vec![(2, true)],
    );
    db.run("revenue by city", &agg);
    db.run("items per user (merge join, filtered)", &Plan::Aggregate {
        input: Box::new(Plan::Filter(Box::new(join(false)), Pred::Not(Box::new(Pred::Eq(3, s("mug")))))),
        group: vec![5, 3],
        aggs: vec![(AggFn::Count, 0)],
    });
    db.run("self-join same city", &Plan::Limit(
        Box::new(Plan::Filter(
            Box::new(Plan::HashJoin { left: Box::new(Plan::Scan("users".into())), right: Box::new(Plan::Scan("users".into())), lkey: 2, rkey: 2 }),
            Pred::And(Box::new(Pred::Lt(0, Datum::Int(6))), Box::new(Pred::Not(Box::new(Pred::ColEq(0, 4))))),
        )),
        6,
    ));
    let orphan_orders = db.exec(&Plan::Scan("orders".into())).filter(|o| !db.tables["users"].rows.iter().any(|u| u[0] == o[1])).count();
    let null_ages = db.exec(&Plan::Filter(Box::new(Plan::Scan("users".into())), Pred::Eq(3, Datum::Null))).count();
    println!("orphan orders {} null ages {}", orphan_orders, null_ages);
    println!("{:?} {:?}", Datum::Int(3).partial_cmp(&Datum::Int(4)), s("a").partial_cmp(&Datum::Int(1)));
}
