use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::fmt;

#[derive(Debug, Clone, PartialEq, PartialOrd)]
enum Value { Null, Int(i64), Float(f64), Text(String), Bool(bool) }

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Value::Null => f.pad("NULL"),
            Value::Int(i) => fmt::Display::fmt(i, f),
            Value::Float(x) => if let Some(p) = f.precision() { write!(f, "{:.*}", p, x) } else { write!(f, "{:.2}", x) },
            Value::Text(s) => f.pad(s),
            Value::Bool(b) => f.pad(if *b { "yes" } else { "no" }),
        }
    }
}

impl Value {
    fn as_f64(&self) -> Option<f64> { match self { Value::Int(i) => Some(*i as f64), Value::Float(f) => Some(*f), _ => None } }
    fn cmp_total(&self, o: &Value) -> Ordering {
        fn rank(v: &Value) -> u8 { match v { Value::Null => 0, Value::Bool(_) => 1, Value::Int(_) | Value::Float(_) => 2, Value::Text(_) => 3 } }
        match (self.as_f64(), o.as_f64()) {
            (Some(a), Some(b)) => a.total_cmp(&b),
            _ => rank(self).cmp(&rank(o)).then_with(|| self.partial_cmp(o).unwrap_or(Ordering::Equal)),
        }
    }
}

type Row = BTreeMap<&'static str, Value>;

enum Pred { Eq(&'static str, Value), Gt(&'static str, f64), Like(&'static str, &'static str), And(Box<Pred>, Box<Pred>), Or(Box<Pred>, Box<Pred>), Not(Box<Pred>), IsNull(&'static str) }

impl Pred {
    fn test(&self, r: &Row) -> bool {
        match self {
            Pred::Eq(c, v) => r.get(c) == Some(v),
            Pred::Gt(c, x) => r.get(c).and_then(Value::as_f64).map_or(false, |v| v > *x),
            Pred::Like(c, pat) => match r.get(c) { Some(Value::Text(s)) => if let Some(p) = pat.strip_suffix('%') { s.starts_with(p) } else if let Some(p) = pat.strip_prefix('%') { s.ends_with(p) } else { s == pat }, _ => false },
            Pred::And(a, b) => a.test(r) && b.test(r),
            Pred::Or(a, b) => a.test(r) || b.test(r),
            Pred::Not(a) => !a.test(r),
            Pred::IsNull(c) => matches!(r.get(c), None | Some(Value::Null)),
        }
    }
}

enum Agg { Count, Sum(&'static str), Avg(&'static str), Max(&'static str) }

struct Query<'a> { rows: &'a [Row], filter: Option<Pred>, order: Vec<(&'static str, bool)>, limit: Option<usize> }

impl<'a> Query<'a> {
    fn from(rows: &'a [Row]) -> Self { Query { rows, filter: None, order: vec![], limit: None } }
    fn filter(mut self, p: Pred) -> Self { self.filter = Some(match self.filter.take() { Some(old) => Pred::And(Box::new(old), Box::new(p)), None => p }); self }
    fn order_by(mut self, col: &'static str, desc: bool) -> Self { self.order.push((col, desc)); self }
    fn limit(mut self, n: usize) -> Self { self.limit = Some(n); self }
    fn run(&self) -> Vec<&'a Row> {
        let mut out: Vec<&Row> = self.rows.iter().filter(|r| self.filter.as_ref().map_or(true, |p| p.test(r))).collect();
        out.sort_by(|a, b| {
            for (c, desc) in &self.order {
                let o = a.get(c).unwrap_or(&Value::Null).cmp_total(b.get(c).unwrap_or(&Value::Null));
                let o = if *desc { o.reverse() } else { o };
                if o != Ordering::Equal { return o; }
            }
            Ordering::Equal
        });
        if let Some(n) = self.limit { out.truncate(n); }
        out
    }
    fn group(&self, key: &'static str, aggs: &[Agg]) -> Vec<(Value, Vec<Value>)> {
        let mut groups: Vec<(Value, Vec<&Row>)> = vec![];
        for r in self.run() {
            let k = r.get(key).cloned().unwrap_or(Value::Null);
            match groups.iter_mut().find(|(gk, _)| *gk == k) { Some((_, v)) => v.push(r), None => groups.push((k, vec![r])) }
        }
        groups.sort_by(|a, b| a.0.cmp_total(&b.0));
        groups.into_iter().map(|(k, rs)| {
            let vals = aggs.iter().map(|a| match a {
                Agg::Count => Value::Int(rs.len() as i64),
                Agg::Sum(c) => Value::Float(rs.iter().filter_map(|r| r.get(c)?.as_f64()).sum()),
                Agg::Avg(c) => { let v: Vec<f64> = rs.iter().filter_map(|r| r.get(c)?.as_f64()).collect(); if v.is_empty() { Value::Null } else { Value::Float(v.iter().sum::<f64>() / v.len() as f64) } }
                Agg::Max(c) => rs.iter().filter_map(|r| r.get(c)).max_by(|a, b| a.cmp_total(b)).cloned().unwrap_or(Value::Null),
            }).collect();
            (k, vals)
        }).collect()
    }
}

fn row(name: &str, dept: &str, salary: Option<f64>, age: i64, remote: bool) -> Row {
    let mut r = Row::new();
    r.insert("name", Value::Text(name.into()));
    r.insert("dept", Value::Text(dept.into()));
    r.insert("salary", salary.map_or(Value::Null, Value::Float));
    r.insert("age", Value::Int(age));
    r.insert("remote", Value::Bool(remote));
    r
}

fn print_rows(title: &str, rows: &[&Row], cols: &[&'static str]) {
    println!("-- {}", title);
    println!("{}", cols.iter().map(|c| format!("{:<10}", c)).collect::<Vec<_>>().join("|"));
    for r in rows { println!("{}", cols.iter().map(|c| format!("{:<10.1}", r.get(c).unwrap_or(&Value::Null))).collect::<Vec<_>>().join("|")); }
}

fn main() {
    let rows = vec![
        row("alice", "eng", Some(120.5), 34, true), row("bob", "eng", Some(95.0), 45, false), row("carol", "sales", Some(70.25), 29, true),
        row("dave", "sales", None, 52, false), row("erin", "eng", Some(130.0), 38, false), row("frank", "ops", Some(88.8), 41, true),
        row("grace", "ops", Some(88.8), 23, false), row("heidi", "sales", Some(99.9), 31, false), row("ivan", "hr", None, 60, true),
    ];
    let cols = ["name", "dept", "salary", "age", "remote"];
    print_rows("all by salary desc, name", &Query::from(&rows).order_by("salary", true).order_by("name", false).run(), &cols);
    print_rows("eng or remote, age>30, top 3 by age", &Query::from(&rows).filter(Pred::Or(Box::new(Pred::Eq("dept", Value::Text("eng".into()))), Box::new(Pred::Eq("remote", Value::Bool(true))))).filter(Pred::Gt("age", 30.0)).order_by("age", false).limit(3).run(), &cols);
    print_rows("name like %e, not null salary", &Query::from(&rows).filter(Pred::Like("name", "%e")).filter(Pred::Not(Box::new(Pred::IsNull("salary")))).run(), &cols[..3]);
    print_rows("null salary", &Query::from(&rows).filter(Pred::IsNull("salary")).run(), &["name", "salary"]);
    for (k, vals) in Query::from(&rows).group("dept", &[Agg::Count, Agg::Sum("salary"), Agg::Avg("salary"), Agg::Max("age")]) {
        println!("{:>6}: {}", k, vals.iter().map(|v| format!("{:>8.3}", v)).collect::<Vec<_>>().join(" "));
    }
    for (k, vals) in Query::from(&rows).filter(Pred::Like("dept", "s%")).group("remote", &[Agg::Count, Agg::Max("name")]) {
        println!("remote={}: {:?}", k, vals);
    }
    println!("{:?}", Value::Int(3).partial_cmp(&Value::Float(1.0)));
    println!("{:?}", Value::Int(3).cmp_total(&Value::Float(3.5)));
}
