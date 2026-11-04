// A tiny relational query engine: typed values, rows, a SQL-ish parser for
// SELECT ... FROM ... [JOIN ... ON ...] [WHERE ...] [GROUP BY ...]
// [ORDER BY ...] [LIMIT n], evaluated with iterator pipelines and boxed
// closures. Includes aggregates (COUNT, SUM, AVG, MIN, MAX) and NULLs.
use std::cmp::Ordering;
use std::collections::{BTreeMap, HashMap};
use std::fmt;

#[derive(Clone, Debug, PartialEq)]
enum Val { Null, Int(i64), Real(f64), Text(String), Bool(bool) }

impl Val {
    fn cmp_sql(&self, o: &Val) -> Option<Ordering> {
        match (self, o) {
            (Val::Null, _) | (_, Val::Null) => None,
            (Val::Int(a), Val::Int(b)) => Some(a.cmp(b)),
            (Val::Int(a), Val::Real(b)) => (*a as f64).partial_cmp(b),
            (Val::Real(a), Val::Int(b)) => a.partial_cmp(&(*b as f64)),
            (Val::Real(a), Val::Real(b)) => a.partial_cmp(b),
            (Val::Text(a), Val::Text(b)) => Some(a.cmp(b)),
            (Val::Bool(a), Val::Bool(b)) => Some(a.cmp(b)),
            _ => None,
        }
    }
    fn sort_key_cmp(&self, o: &Val) -> Ordering {
        // NULLs first, then by value
        match (self, o) {
            (Val::Null, Val::Null) => Ordering::Equal,
            (Val::Null, _) => Ordering::Less,
            (_, Val::Null) => Ordering::Greater,
            _ => self.cmp_sql(o).unwrap_or(Ordering::Equal),
        }
    }
    fn as_f64(&self) -> Option<f64> {
        match self { Val::Int(i) => Some(*i as f64), Val::Real(r) => Some(*r), _ => None }
    }
}

impl fmt::Display for Val {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Val::Null => f.pad("NULL"),
            Val::Int(i) => f.pad(&i.to_string()),
            Val::Real(r) => f.pad(&format!("{:.2}", r)),
            Val::Text(s) => f.pad(s),
            Val::Bool(b) => f.pad(if *b { "TRUE" } else { "FALSE" }),
        }
    }
}

#[derive(Clone)]
struct Table { name: String, cols: Vec<String>, rows: Vec<Vec<Val>> }

#[derive(Debug, Clone)]
enum Expr {
    Col(String),
    Lit(Val),
    Bin(String, Box<Expr>, Box<Expr>),
    Not(Box<Expr>),
    IsNull(Box<Expr>, bool),
    Agg(String, Option<Box<Expr>>),
}

#[derive(Debug)]
struct Query {
    select: Vec<(Expr, String)>,
    from: String,
    join: Option<(String, Expr)>,
    filter: Option<Expr>,
    group_by: Vec<String>,
    order_by: Vec<(String, bool)>,
    limit: Option<usize>,
}

fn lex(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let cs: Vec<char> = s.chars().collect();
    let mut i = 0;
    while i < cs.len() {
        let c = cs[i];
        if c.is_whitespace() { i += 1; continue; }
        if c == '\'' {
            let mut j = i + 1;
            while j < cs.len() && cs[j] != '\'' { j += 1; }
            out.push(cs[i..=j.min(cs.len() - 1)].iter().collect());
            i = j + 1;
            continue;
        }
        if c.is_alphanumeric() || c == '_' || c == '.' {
            let st = i;
            while i < cs.len() && (cs[i].is_alphanumeric() || cs[i] == '_' || cs[i] == '.') { i += 1; }
            out.push(cs[st..i].iter().collect());
            continue;
        }
        let two: String = cs[i..(i + 2).min(cs.len())].iter().collect();
        if ["<=", ">=", "<>", "!="].contains(&two.as_str()) { out.push(two); i += 2; continue; }
        out.push(c.to_string());
        i += 1;
    }
    out
}

struct Parser { t: Vec<String>, i: usize }

impl Parser {
    fn peek_kw(&self, kw: &str) -> bool { self.t.get(self.i).map_or(false, |s| s.eq_ignore_ascii_case(kw)) }
    fn kw(&mut self, kw: &str) -> Result<(), String> {
        if self.peek_kw(kw) { self.i += 1; Ok(()) } else { Err(format!("expected {} at {:?}", kw, self.t.get(self.i))) }
    }
    fn next(&mut self) -> Result<String, String> {
        let t = self.t.get(self.i).cloned().ok_or("unexpected end of query")?;
        self.i += 1;
        Ok(t)
    }
    fn query(&mut self) -> Result<Query, String> {
        self.kw("SELECT")?;
        let mut select = Vec::new();
        loop {
            let e = self.expr()?;
            let alias = if self.peek_kw("AS") { self.i += 1; self.next()? } else { expr_name(&e) };
            select.push((e, alias));
            if self.t.get(self.i).map(|s| s.as_str()) == Some(",") { self.i += 1; } else { break; }
        }
        self.kw("FROM")?;
        let from = self.next()?;
        let join = if self.peek_kw("JOIN") {
            self.i += 1;
            let t = self.next()?;
            self.kw("ON")?;
            Some((t, self.expr()?))
        } else { None };
        let filter = if self.peek_kw("WHERE") { self.i += 1; Some(self.expr()?) } else { None };
        let mut group_by = vec![];
        if self.peek_kw("GROUP") {
            self.i += 1;
            self.kw("BY")?;
            loop {
                group_by.push(self.next()?);
                if self.t.get(self.i).map(|s| s.as_str()) == Some(",") { self.i += 1; } else { break; }
            }
        }
        let mut order_by = vec![];
        if self.peek_kw("ORDER") {
            self.i += 1;
            self.kw("BY")?;
            loop {
                let c = self.next()?;
                let desc = if self.peek_kw("DESC") { self.i += 1; true } else { if self.peek_kw("ASC") { self.i += 1; } false };
                order_by.push((c, desc));
                if self.t.get(self.i).map(|s| s.as_str()) == Some(",") { self.i += 1; } else { break; }
            }
        }
        let limit = if self.peek_kw("LIMIT") { self.i += 1; Some(self.next()?.parse().map_err(|e| format!("bad limit: {}", e))?) } else { None };
        if self.i < self.t.len() { return Err(format!("trailing tokens: {:?}", &self.t[self.i..])); }
        Ok(Query { select, from, join, filter, group_by, order_by, limit })
    }
    fn expr(&mut self) -> Result<Expr, String> { self.or() }
    fn or(&mut self) -> Result<Expr, String> {
        let mut l = self.and()?;
        while self.peek_kw("OR") { self.i += 1; l = Expr::Bin("OR".into(), Box::new(l), Box::new(self.and()?)); }
        Ok(l)
    }
    fn and(&mut self) -> Result<Expr, String> {
        let mut l = self.not()?;
        while self.peek_kw("AND") { self.i += 1; l = Expr::Bin("AND".into(), Box::new(l), Box::new(self.not()?)); }
        Ok(l)
    }
    fn not(&mut self) -> Result<Expr, String> {
        if self.peek_kw("NOT") { self.i += 1; return Ok(Expr::Not(Box::new(self.not()?))); }
        self.cmp()
    }
    fn cmp(&mut self) -> Result<Expr, String> {
        let l = self.add()?;
        if self.peek_kw("IS") {
            self.i += 1;
            let neg = if self.peek_kw("NOT") { self.i += 1; true } else { false };
            self.kw("NULL")?;
            return Ok(Expr::IsNull(Box::new(l), neg));
        }
        match self.t.get(self.i).map(|s| s.as_str()) {
            Some(op @ ("=" | "<" | ">" | "<=" | ">=" | "<>" | "!=")) => {
                let op = op.to_string();
                self.i += 1;
                Ok(Expr::Bin(op, Box::new(l), Box::new(self.add()?)))
            }
            _ => Ok(l),
        }
    }
    fn add(&mut self) -> Result<Expr, String> {
        let mut l = self.mul()?;
        while let Some(op @ ("+" | "-")) = self.t.get(self.i).map(|s| s.as_str()) {
            let op = op.to_string();
            self.i += 1;
            l = Expr::Bin(op, Box::new(l), Box::new(self.mul()?));
        }
        Ok(l)
    }
    fn mul(&mut self) -> Result<Expr, String> {
        let mut l = self.atom()?;
        while let Some(op @ ("*" | "/")) = self.t.get(self.i).map(|s| s.as_str()) {
            let op = op.to_string();
            self.i += 1;
            l = Expr::Bin(op, Box::new(l), Box::new(self.atom()?));
        }
        Ok(l)
    }
    fn atom(&mut self) -> Result<Expr, String> {
        let t = self.next()?;
        let up = t.to_ascii_uppercase();
        if ["COUNT", "SUM", "AVG", "MIN", "MAX"].contains(&up.as_str()) && self.t.get(self.i).map(|s| s.as_str()) == Some("(") {
            self.i += 1;
            let arg = if self.t.get(self.i).map(|s| s.as_str()) == Some("*") { self.i += 1; None } else { Some(Box::new(self.expr()?)) };
            if self.next()? != ")" { return Err("expected )".into()); }
            return Ok(Expr::Agg(up, arg));
        }
        Ok(match t.as_str() {
            "(" => { let e = self.expr()?; if self.next()? != ")" { return Err("expected )".into()); } e }
            "*" => Expr::Col("*".into()),
            s if s.starts_with('\'') => Expr::Lit(Val::Text(s.trim_matches('\'').to_string())),
            s if up == "NULL" => { let _ = s; Expr::Lit(Val::Null) }
            s if up == "TRUE" || up == "FALSE" => { let _ = s; Expr::Lit(Val::Bool(up == "TRUE")) }
            s if s.chars().next().unwrap().is_ascii_digit() => {
                if s.contains('.') { Expr::Lit(Val::Real(s.parse().map_err(|_| format!("bad number {}", s))?)) }
                else { Expr::Lit(Val::Int(s.parse().map_err(|_| format!("bad number {}", s))?)) }
            }
            s if s.chars().next().unwrap().is_alphabetic() => Expr::Col(s.to_string()),
            s => return Err(format!("unexpected token {:?}", s)),
        })
    }
}

fn expr_name(e: &Expr) -> String {
    match e {
        Expr::Col(c) => c.clone(),
        Expr::Lit(v) => v.to_string(),
        Expr::Agg(f, None) => format!("{}(*)", f),
        Expr::Agg(f, Some(a)) => format!("{}({})", f, expr_name(a)),
        Expr::Bin(op, a, b) => format!("{}{}{}", expr_name(a), op, expr_name(b)),
        Expr::Not(a) => format!("NOT {}", expr_name(a)),
        Expr::IsNull(a, n) => format!("{} IS {}NULL", expr_name(a), if *n { "NOT " } else { "" }),
    }
}

type Row<'a> = (&'a [String], Vec<Val>);

fn lookup(cols: &[String], row: &[Val], name: &str) -> Result<Val, String> {
    if let Some(i) = cols.iter().position(|c| c == name) { return Ok(row[i].clone()); }
    let matches: Vec<usize> = cols.iter().enumerate().filter(|(_, c)| c.rsplit('.').next() == Some(name)).map(|(i, _)| i).collect();
    match matches.as_slice() {
        [i] => Ok(row[*i].clone()),
        [] => Err(format!("unknown column {}", name)),
        _ => Err(format!("ambiguous column {}", name)),
    }
}

fn eval(e: &Expr, cols: &[String], row: &[Val], group: Option<&[Vec<Val>]>) -> Result<Val, String> {
    Ok(match e {
        Expr::Col(c) => lookup(cols, row, c)?,
        Expr::Lit(v) => v.clone(),
        Expr::Not(a) => match eval(a, cols, row, group)? { Val::Bool(b) => Val::Bool(!b), Val::Null => Val::Null, v => return Err(format!("NOT on {:?}", v)) },
        Expr::IsNull(a, neg) => Val::Bool((eval(a, cols, row, group)? == Val::Null) != *neg),
        Expr::Agg(f, arg) => {
            let rows = group.ok_or_else(|| format!("aggregate {} outside GROUP BY context", f))?;
            let vals: Vec<Val> = match arg {
                None => rows.iter().map(|_| Val::Int(1)).collect(),
                Some(a) => rows.iter().map(|r| eval(a, cols, r, None)).collect::<Result<Vec<_>, _>>()?.into_iter().filter(|v| *v != Val::Null).collect(),
            };
            match f.as_str() {
                "COUNT" => Val::Int(vals.len() as i64),
                "SUM" if vals.is_empty() => Val::Null,
                "SUM" => if vals.iter().all(|v| matches!(v, Val::Int(_))) {
                    Val::Int(vals.iter().map(|v| if let Val::Int(i) = v { *i } else { 0 }).sum())
                } else { Val::Real(vals.iter().filter_map(Val::as_f64).sum()) },
                "AVG" => if vals.is_empty() { Val::Null } else { Val::Real(vals.iter().filter_map(Val::as_f64).sum::<f64>() / vals.len() as f64) },
                "MIN" => vals.into_iter().min_by(|a, b| a.sort_key_cmp(b)).unwrap_or(Val::Null),
                "MAX" => vals.into_iter().max_by(|a, b| a.sort_key_cmp(b)).unwrap_or(Val::Null),
                _ => unreachable!(),
            }
        }
        Expr::Bin(op, a, b) => {
            let (x, y) = (eval(a, cols, row, group)?, eval(b, cols, row, group)?);
            match op.as_str() {
                "AND" => match (&x, &y) { (Val::Bool(false), _) | (_, Val::Bool(false)) => Val::Bool(false), (Val::Bool(true), Val::Bool(true)) => Val::Bool(true), _ => Val::Null },
                "OR" => match (&x, &y) { (Val::Bool(true), _) | (_, Val::Bool(true)) => Val::Bool(true), (Val::Bool(false), Val::Bool(false)) => Val::Bool(false), _ => Val::Null },
                "+" | "-" | "*" | "/" => match (&x, &y) {
                    (Val::Null, _) | (_, Val::Null) => Val::Null,
                    (Val::Int(p), Val::Int(q)) => match op.as_str() {
                        "+" => Val::Int(p + q), "-" => Val::Int(p - q), "*" => Val::Int(p * q),
                        _ => if *q == 0 { Val::Null } else { Val::Int(p / q) },
                    },
                    _ => {
                        let (p, q) = (x.as_f64().ok_or("non-numeric")?, y.as_f64().ok_or("non-numeric")?);
                        Val::Real(match op.as_str() { "+" => p + q, "-" => p - q, "*" => p * q, _ => p / q })
                    }
                },
                _ => match x.cmp_sql(&y) {
                    None => Val::Null,
                    Some(o) => Val::Bool(match op.as_str() {
                        "=" => o == Ordering::Equal, "<>" | "!=" => o != Ordering::Equal,
                        "<" => o == Ordering::Less, ">" => o == Ordering::Greater,
                        "<=" => o != Ordering::Greater, ">=" => o != Ordering::Less, _ => unreachable!(),
                    }),
                },
            }
        }
    })
}

fn has_agg(e: &Expr) -> bool {
    match e {
        Expr::Agg(..) => true,
        Expr::Bin(_, a, b) => has_agg(a) || has_agg(b),
        Expr::Not(a) | Expr::IsNull(a, _) => has_agg(a),
        _ => false,
    }
}

fn run(db: &HashMap<String, Table>, sql: &str) -> Result<(Vec<String>, Vec<Vec<Val>>), String> {
    let q = Parser { t: lex(sql), i: 0 }.query()?;
    let base = db.get(&q.from).ok_or_else(|| format!("no such table {}", q.from))?;
    let mut cols: Vec<String> = base.cols.iter().map(|c| format!("{}.{}", base.name, c)).collect();
    let mut rows: Vec<Vec<Val>> = base.rows.clone();
    if let Some((jt, on)) = &q.join {
        let other = db.get(jt).ok_or_else(|| format!("no such table {}", jt))?;
        let jcols: Vec<String> = cols.iter().cloned().chain(other.cols.iter().map(|c| format!("{}.{}", other.name, c))).collect();
        let mut out = Vec::new();
        for l in &rows {
            for r in &other.rows {
                let combined: Vec<Val> = l.iter().chain(r.iter()).cloned().collect();
                if eval(on, &jcols, &combined, None)? == Val::Bool(true) { out.push(combined); }
            }
        }
        cols = jcols;
        rows = out;
    }
    if let Some(f) = &q.filter {
        let mut kept = Vec::new();
        for r in rows {
            if eval(f, &cols, &r, None)? == Val::Bool(true) { kept.push(r); }
        }
        rows = kept;
    }
    let star = q.select.iter().any(|(e, _)| matches!(e, Expr::Col(c) if c == "*"));
    let headers: Vec<String> = if star { cols.clone() } else { q.select.iter().map(|(_, a)| a.clone()).collect() };
    let aggregate = !q.group_by.is_empty() || q.select.iter().any(|(e, _)| has_agg(e));
    let mut result: Vec<Vec<Val>> = if aggregate {
        let mut groups: BTreeMap<Vec<String>, Vec<Vec<Val>>> = BTreeMap::new();
        for r in rows {
            let key = q.group_by.iter().map(|g| lookup(&cols, &r, g).map(|v| format!("{:?}", v))).collect::<Result<Vec<_>, _>>()?;
            groups.entry(key).or_default().push(r);
        }
        if groups.is_empty() && q.group_by.is_empty() { groups.insert(vec![], vec![]); }
        let mut out = Vec::new();
        for (_, g) in &groups {
            let first = g.first().cloned().unwrap_or_else(|| vec![Val::Null; cols.len()]);
            out.push(q.select.iter().map(|(e, _)| eval(e, &cols, &first, Some(g))).collect::<Result<Vec<_>, _>>()?);
        }
        out
    } else if star {
        rows
    } else {
        rows.iter().map(|r| q.select.iter().map(|(e, _)| eval(e, &cols, r, None)).collect()).collect::<Result<Vec<_>, _>>()?
    };
    if !q.order_by.is_empty() {
        let idx: Vec<(usize, bool)> = q.order_by.iter().map(|(c, d)| {
            headers.iter().position(|h| h == c || h.rsplit('.').next() == Some(c)).map(|i| (i, *d)).ok_or_else(|| format!("cannot order by {}", c))
        }).collect::<Result<_, _>>()?;
        result.sort_by(|a, b| idx.iter().fold(Ordering::Equal, |acc, &(i, desc)| acc.then_with(|| {
            let o = a[i].sort_key_cmp(&b[i]);
            if desc { o.reverse() } else { o }
        })));
    }
    if let Some(n) = q.limit { result.truncate(n); }
    Ok((headers, result))
}

fn show(db: &HashMap<String, Table>, sql: &str) {
    println!("sql> {}", sql);
    match run(db, sql) {
        Err(e) => println!("ERROR: {}\n", e),
        Ok((h, rows)) => {
            let widths: Vec<usize> = (0..h.len()).map(|i| rows.iter().map(|r| r[i].to_string().chars().count()).chain(std::iter::once(h[i].len())).max().unwrap()).collect();
            println!("{}", h.iter().zip(&widths).map(|(c, w)| format!("{:<w$}", c, w = *w)).collect::<Vec<_>>().join(" | "));
            println!("{}", widths.iter().map(|w| "-".repeat(*w)).collect::<Vec<_>>().join("-+-"));
            for r in &rows {
                println!("{}", r.iter().zip(&widths).map(|(v, w)| match v {
                    Val::Int(_) | Val::Real(_) => format!("{:>w$}", v, w = *w),
                    _ => format!("{:<w$}", v, w = *w),
                }).collect::<Vec<_>>().join(" | "));
            }
            println!("({} row{})\n", rows.len(), if rows.len() == 1 { "" } else { "s" });
        }
    }
}

fn main() {
    let t = |s: &str| Val::Text(s.to_string());
    let mut db = HashMap::new();
    db.insert("emp".to_string(), Table {
        name: "emp".into(),
        cols: vec!["id".into(), "name".into(), "dept".into(), "salary".into(), "manager".into()],
        rows: vec![
            vec![Val::Int(1), t("Ada"), Val::Int(10), Val::Int(185000), Val::Null],
            vec![Val::Int(2), t("Brian"), Val::Int(10), Val::Int(120000), Val::Int(1)],
            vec![Val::Int(3), t("Chen"), Val::Int(20), Val::Int(99000), Val::Int(1)],
            vec![Val::Int(4), t("Dana"), Val::Int(20), Val::Int(105500), Val::Int(3)],
            vec![Val::Int(5), t("Eve"), Val::Int(30), Val::Null, Val::Int(1)],
            vec![Val::Int(6), t("Femi"), Val::Int(10), Val::Int(120000), Val::Int(2)],
            vec![Val::Int(7), t("Gus"), Val::Null, Val::Int(45000), Val::Null],
        ],
    });
    db.insert("dept".to_string(), Table {
        name: "dept".into(),
        cols: vec!["id".into(), "title".into(), "budget".into()],
        rows: vec![
            vec![Val::Int(10), t("Engineering"), Val::Real(1.5e6)],
            vec![Val::Int(20), t("Research"), Val::Real(750000.0)],
            vec![Val::Int(30), t("Sales"), Val::Real(300000.0)],
            vec![Val::Int(40), t("Legal"), Val::Null],
        ],
    });
    let queries = [
        "SELECT * FROM dept",
        "SELECT name, salary FROM emp WHERE salary >= 100000 ORDER BY salary DESC, name",
        "SELECT name, salary * 2 AS double, salary / 0 FROM emp WHERE dept = 20",
        "SELECT name FROM emp WHERE manager IS NULL OR salary < 50000",
        "SELECT name, dept FROM emp WHERE NOT dept = 10 ORDER BY dept DESC",
        "SELECT COUNT(*), COUNT(salary), SUM(salary), AVG(salary), MIN(name), MAX(salary) FROM emp",
        "SELECT dept, COUNT(*) AS n, SUM(salary) AS total, AVG(salary) AS avg FROM emp GROUP BY dept ORDER BY total DESC",
        "SELECT emp.name, dept.title, dept.budget FROM emp JOIN dept ON emp.dept = dept.id ORDER BY title, name",
        "SELECT title, COUNT(*) AS headcount, MAX(salary) AS top FROM emp JOIN dept ON emp.dept = dept.id GROUP BY title ORDER BY headcount DESC LIMIT 2",
        "SELECT COUNT(*) FROM emp WHERE salary > 1000000",
        "SELECT name, (salary + 1000) * 2 - 1 AS x FROM emp WHERE salary IS NOT NULL AND (dept = 10 OR dept = 30) LIMIT 2",
        "SELECT id FROM emp JOIN dept ON emp.dept = dept.id",
        "SELECT name FROM nope",
        "SELECT name FROM emp ORDER BY wage",
        "SELECT name FROM emp LIMIT x",
        "SELECT name emp",
        "SELECT name FROM emp WHERE",
    ];
    for q in queries { show(&db, q); }
}
