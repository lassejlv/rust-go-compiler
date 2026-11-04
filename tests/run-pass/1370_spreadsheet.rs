// A spreadsheet engine: A1-style references and ranges, a formula parser
// (precedence climbing, functions, comparisons, string concatenation),
// a dependency graph with incremental recalculation in topological order,
// cycle detection with error propagation, and a text renderer.
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
struct CellRef {
    col: u32,
    row: u32,
}

impl CellRef {
    fn parse(s: &str) -> Option<CellRef> {
        let s = s.trim_start_matches('$');
        let letters: String = s.chars().take_while(|c| c.is_ascii_alphabetic()).collect();
        let digits = s[letters.len()..].trim_start_matches('$');
        if letters.is_empty() || digits.is_empty() || !digits.chars().all(|c| c.is_ascii_digit()) {
            return None;
        }
        let col = letters.to_ascii_uppercase().bytes().fold(0u32, |acc, b| acc * 26 + (b - b'A' + 1) as u32) - 1;
        let row = digits.parse::<u32>().ok()?.checked_sub(1)?;
        Some(CellRef { col, row })
    }
}

impl fmt::Display for CellRef {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut c = self.col + 1;
        let mut name = Vec::new();
        while c > 0 {
            let r = (c - 1) % 26;
            name.push((b'A' + r as u8) as char);
            c = (c - 1) / 26;
        }
        name.reverse();
        write!(f, "{}{}", name.into_iter().collect::<String>(), self.row + 1)
    }
}

#[derive(Debug, Clone, PartialEq)]
enum Value {
    Empty,
    Num(f64),
    Text(String),
    Bool(bool),
    Error(ErrKind),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ErrKind {
    Div0,
    Ref,
    Value,
    Cycle,
    Name,
    Parse,
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Value::Empty => Ok(()),
            Value::Num(n) if n.fract() == 0.0 && n.abs() < 1e15 => write!(f, "{}", *n as i64),
            Value::Num(n) => write!(f, "{:.4}", n),
            Value::Text(s) => f.write_str(s),
            Value::Bool(b) => f.write_str(if *b { "TRUE" } else { "FALSE" }),
            Value::Error(e) => write!(
                f,
                "#{}",
                match e {
                    ErrKind::Div0 => "DIV/0!",
                    ErrKind::Ref => "REF!",
                    ErrKind::Value => "VALUE!",
                    ErrKind::Cycle => "CYCLE!",
                    ErrKind::Name => "NAME?",
                    ErrKind::Parse => "PARSE!",
                }
            ),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
enum Expr {
    Num(f64),
    Str(String),
    Ref(CellRef),
    Range(CellRef, CellRef),
    Unary(char, Box<Expr>),
    Bin(String, Box<Expr>, Box<Expr>),
    Call(String, Vec<Expr>),
}

struct Parser<'a> {
    toks: Vec<&'a str>,
    pos: usize,
}

fn tokenize(s: &str) -> Result<Vec<&str>, String> {
    let mut out = Vec::new();
    let b = s.as_bytes();
    let mut i = 0;
    while i < b.len() {
        let c = b[i] as char;
        if c.is_whitespace() {
            i += 1;
            continue;
        }
        let start = i;
        if c == '"' {
            i += 1;
            while i < b.len() && b[i] != b'"' {
                i += 1;
            }
            if i == b.len() {
                return Err("unterminated string".into());
            }
            i += 1;
        } else if c.is_ascii_alphanumeric() || c == '.' || c == '$' || c == '_' {
            while i < b.len() && ((b[i] as char).is_ascii_alphanumeric() || b[i] == b'.' || b[i] == b'$' || b[i] == b'_') {
                i += 1;
            }
        } else if (c == '<' || c == '>') && i + 1 < b.len() && (b[i + 1] == b'=' || b[i + 1] == b'>') {
            i += 2;
        } else if "+-*/^&(),:=<>%".contains(c) {
            i += 1;
        } else {
            return Err(format!("unexpected {:?}", c));
        }
        out.push(&s[start..i]);
    }
    Ok(out)
}

impl<'a> Parser<'a> {
    fn peek(&self) -> Option<&'a str> {
        self.toks.get(self.pos).copied()
    }
    fn bump(&mut self) -> Option<&'a str> {
        let t = self.peek();
        self.pos += 1;
        t
    }
    fn prec(op: &str) -> Option<(u8, bool)> {
        Some(match op {
            "=" | "<>" | "<" | ">" | "<=" | ">=" => (1, false),
            "&" => (2, false),
            "+" | "-" => (3, false),
            "*" | "/" => (4, false),
            "^" => (5, true),
            _ => return None,
        })
    }
    fn expr(&mut self, min: u8) -> Result<Expr, String> {
        let mut lhs = self.unary()?;
        while let Some(op) = self.peek() {
            let Some((p, right)) = Self::prec(op) else { break };
            if p < min {
                break;
            }
            self.bump();
            let rhs = self.expr(if right { p } else { p + 1 })?;
            lhs = Expr::Bin(op.to_string(), Box::new(lhs), Box::new(rhs));
        }
        Ok(lhs)
    }
    fn unary(&mut self) -> Result<Expr, String> {
        match self.peek() {
            Some("-") => {
                self.bump();
                Ok(Expr::Unary('-', Box::new(self.unary()?)))
            }
            Some("+") => {
                self.bump();
                self.unary()
            }
            _ => {
                let e = self.atom()?;
                if self.peek() == Some("%") {
                    self.bump();
                    return Ok(Expr::Bin("/".into(), Box::new(e), Box::new(Expr::Num(100.0))));
                }
                Ok(e)
            }
        }
    }
    fn atom(&mut self) -> Result<Expr, String> {
        let t = self.bump().ok_or("unexpected end")?;
        if t == "(" {
            let e = self.expr(0)?;
            return match self.bump() {
                Some(")") => Ok(e),
                other => Err(format!("expected ), got {:?}", other)),
            };
        }
        if let Some(s) = t.strip_prefix('"') {
            return Ok(Expr::Str(s.trim_end_matches('"').to_string()));
        }
        if let Ok(n) = t.parse::<f64>() {
            return Ok(Expr::Num(n));
        }
        if self.peek() == Some("(") {
            self.bump();
            let mut args = Vec::new();
            if self.peek() != Some(")") {
                loop {
                    args.push(self.expr(0)?);
                    match self.bump() {
                        Some(",") => continue,
                        Some(")") => break,
                        other => return Err(format!("expected , or ), got {:?}", other)),
                    }
                }
            } else {
                self.bump();
            }
            return Ok(Expr::Call(t.to_ascii_uppercase(), args));
        }
        if let Some(a) = CellRef::parse(t) {
            if self.peek() == Some(":") {
                self.bump();
                let b = self.bump().and_then(CellRef::parse).ok_or("bad range end")?;
                let (lo, hi) = (CellRef { col: a.col.min(b.col), row: a.row.min(b.row) }, CellRef { col: a.col.max(b.col), row: a.row.max(b.row) });
                return Ok(Expr::Range(lo, hi));
            }
            return Ok(Expr::Ref(a));
        }
        match t.to_ascii_uppercase().as_str() {
            "TRUE" => Ok(Expr::Num(1.0)),
            "FALSE" => Ok(Expr::Num(0.0)),
            _ => Ok(Expr::Call(format!("#NAME:{}", t), vec![])),
        }
    }
}

fn parse_formula(src: &str) -> Result<Expr, String> {
    let toks = tokenize(src)?;
    let mut p = Parser { toks, pos: 0 };
    let e = p.expr(0)?;
    if p.pos < p.toks.len() {
        return Err(format!("trailing {:?}", p.toks[p.pos]));
    }
    Ok(e)
}

fn deps(e: &Expr, out: &mut BTreeSet<CellRef>) {
    match e {
        Expr::Ref(r) => {
            out.insert(*r);
        }
        Expr::Range(a, b) => {
            for c in a.col..=b.col {
                for r in a.row..=b.row {
                    out.insert(CellRef { col: c, row: r });
                }
            }
        }
        Expr::Unary(_, x) => deps(x, out),
        Expr::Bin(_, a, b) => {
            deps(a, out);
            deps(b, out);
        }
        Expr::Call(_, args) => args.iter().for_each(|a| deps(a, out)),
        Expr::Num(_) | Expr::Str(_) => {}
    }
}

enum Content {
    Literal(Value),
    Formula(String, Result<Expr, String>),
}

#[derive(Default)]
struct Sheet {
    cells: HashMap<CellRef, Content>,
    values: HashMap<CellRef, Value>,
    rdeps: HashMap<CellRef, HashSet<CellRef>>,
    fdeps: HashMap<CellRef, BTreeSet<CellRef>>,
    evals: usize,
}

fn num(v: &Value) -> Result<f64, ErrKind> {
    match v {
        Value::Num(n) => Ok(*n),
        Value::Bool(b) => Ok(*b as u8 as f64),
        Value::Empty => Ok(0.0),
        Value::Text(t) => t.trim().parse().map_err(|_| ErrKind::Value),
        Value::Error(e) => Err(*e),
    }
}

impl Sheet {
    fn set(&mut self, at: &str, input: &str) -> Vec<CellRef> {
        let r = CellRef::parse(at).expect("bad cell");
        // unlink old deps
        if let Some(old) = self.fdeps.remove(&r) {
            for d in old {
                if let Some(s) = self.rdeps.get_mut(&d) {
                    s.remove(&r);
                }
            }
        }
        let content = if let Some(f) = input.strip_prefix('=') {
            let parsed = parse_formula(f);
            if let Ok(e) = &parsed {
                let mut ds = BTreeSet::new();
                deps(e, &mut ds);
                for d in &ds {
                    self.rdeps.entry(*d).or_default().insert(r);
                }
                self.fdeps.insert(r, ds);
            }
            Content::Formula(f.to_string(), parsed)
        } else if input.is_empty() {
            Content::Literal(Value::Empty)
        } else if let Ok(n) = input.parse::<f64>() {
            Content::Literal(Value::Num(n))
        } else if input.eq_ignore_ascii_case("true") || input.eq_ignore_ascii_case("false") {
            Content::Literal(Value::Bool(input.eq_ignore_ascii_case("true")))
        } else {
            Content::Literal(Value::Text(input.to_string()))
        };
        self.cells.insert(r, content);
        self.recalc_from(r)
    }

    /// Recalculates `start` and everything depending on it, in topological
    /// order; cells on a cycle get #CYCLE!. Returns the recalculated cells.
    fn recalc_from(&mut self, start: CellRef) -> Vec<CellRef> {
        // collect affected cells
        let mut affected = BTreeSet::new();
        let mut q = VecDeque::from([start]);
        while let Some(c) = q.pop_front() {
            if affected.insert(c) {
                if let Some(ds) = self.rdeps.get(&c) {
                    let mut v: Vec<_> = ds.iter().copied().collect();
                    v.sort();
                    q.extend(v);
                }
            }
        }
        // Kahn's algorithm restricted to the affected set
        let mut indeg: BTreeMap<CellRef, usize> = affected.iter().map(|c| (*c, 0)).collect();
        for c in &affected {
            if let Some(ds) = self.fdeps.get(c) {
                for d in ds {
                    if affected.contains(d) {
                        *indeg.get_mut(c).unwrap() += 1;
                    }
                }
            }
        }
        let mut ready: VecDeque<CellRef> = indeg.iter().filter(|(_, &n)| n == 0).map(|(c, _)| *c).collect();
        let mut order = Vec::new();
        while let Some(c) = ready.pop_front() {
            order.push(c);
            let mut next: Vec<CellRef> = self.rdeps.get(&c).map(|s| s.iter().copied().filter(|d| affected.contains(d)).collect()).unwrap_or_default();
            next.sort();
            for d in next {
                let e = indeg.get_mut(&d).unwrap();
                *e -= 1;
                if *e == 0 {
                    ready.push_back(d);
                }
            }
        }
        for c in &order {
            let v = self.eval_cell(*c);
            self.values.insert(*c, v);
        }
        for c in affected.iter().filter(|c| !order.contains(c)) {
            self.values.insert(*c, Value::Error(ErrKind::Cycle));
        }
        let mut all: Vec<CellRef> = order;
        all.extend(affected.iter().copied().filter(|c| !all.contains(c)).collect::<Vec<_>>());
        all
    }

    fn get(&self, r: CellRef) -> Value {
        self.values.get(&r).cloned().unwrap_or(Value::Empty)
    }

    fn eval_cell(&mut self, r: CellRef) -> Value {
        self.evals += 1;
        let expr = match self.cells.get(&r) {
            None => return Value::Empty,
            Some(Content::Literal(v)) => return v.clone(),
            Some(Content::Formula(_, Err(_))) => return Value::Error(ErrKind::Parse),
            Some(Content::Formula(_, Ok(e))) => e.clone(),
        };
        self.eval(&expr).unwrap_or_else(Value::Error)
    }

    fn range_values(&self, a: CellRef, b: CellRef) -> Vec<Value> {
        let mut out = vec![];
        for row in a.row..=b.row {
            for col in a.col..=b.col {
                out.push(self.get(CellRef { col, row }));
            }
        }
        out
    }

    fn eval(&self, e: &Expr) -> Result<Value, ErrKind> {
        Ok(match e {
            Expr::Num(n) => Value::Num(*n),
            Expr::Str(s) => Value::Text(s.clone()),
            Expr::Ref(r) => match self.get(*r) {
                Value::Error(k) => return Err(k),
                v => v,
            },
            Expr::Range(..) => return Err(ErrKind::Value),
            Expr::Unary(_, x) => Value::Num(-num(&self.eval(x)?)?),
            Expr::Bin(op, a, b) => {
                let (va, vb) = (self.eval(a)?, self.eval(b)?);
                match op.as_str() {
                    "&" => Value::Text(format!("{}{}", va, vb)),
                    "=" | "<>" | "<" | ">" | "<=" | ">=" => {
                        let ord = match (&va, &vb) {
                            (Value::Text(x), Value::Text(y)) => x.to_lowercase().cmp(&y.to_lowercase()),
                            _ => num(&va)?.partial_cmp(&num(&vb)?).ok_or(ErrKind::Value)?,
                        };
                        use std::cmp::Ordering::*;
                        Value::Bool(match op.as_str() {
                            "=" => ord == Equal,
                            "<>" => ord != Equal,
                            "<" => ord == Less,
                            ">" => ord == Greater,
                            "<=" => ord != Greater,
                            _ => ord != Less,
                        })
                    }
                    _ => {
                        let (x, y) = (num(&va)?, num(&vb)?);
                        Value::Num(match op.as_str() {
                            "+" => x + y,
                            "-" => x - y,
                            "*" => x * y,
                            "/" if y == 0.0 => return Err(ErrKind::Div0),
                            "/" => x / y,
                            "^" => x.powf(y),
                            _ => return Err(ErrKind::Value),
                        })
                    }
                }
            }
            Expr::Call(name, args) => {
                let mut flat: Vec<Value> = Vec::new();
                let mut raw: Vec<Result<Value, ErrKind>> = Vec::new();
                for a in args {
                    match a {
                        Expr::Range(x, y) => flat.extend(self.range_values(*x, *y)),
                        other => {
                            let v = self.eval(other);
                            raw.push(v.clone());
                            flat.push(v.unwrap_or_else(Value::Error));
                        }
                    }
                }
                let nums = || -> Result<Vec<f64>, ErrKind> {
                    flat.iter()
                        .filter(|v| matches!(v, Value::Num(_) | Value::Error(_)))
                        .map(|v| num(v))
                        .collect()
                };
                match name.as_str() {
                    "SUM" => Value::Num(nums()?.iter().sum()),
                    "AVERAGE" => {
                        let v = nums()?;
                        if v.is_empty() {
                            return Err(ErrKind::Div0);
                        }
                        Value::Num(v.iter().sum::<f64>() / v.len() as f64)
                    }
                    "MIN" => Value::Num(nums()?.into_iter().fold(f64::INFINITY, f64::min)),
                    "MAX" => Value::Num(nums()?.into_iter().fold(f64::NEG_INFINITY, f64::max)),
                    "COUNT" => Value::Num(flat.iter().filter(|v| matches!(v, Value::Num(_))).count() as f64),
                    "COUNTA" => Value::Num(flat.iter().filter(|v| **v != Value::Empty).count() as f64),
                    "IF" => match raw.as_slice() {
                        [c, t, f] => {
                            if num(&c.clone()?)? != 0.0 {
                                t.clone()?
                            } else {
                                f.clone()?
                            }
                        }
                        [c, t] => {
                            if num(&c.clone()?)? != 0.0 {
                                t.clone()?
                            } else {
                                Value::Bool(false)
                            }
                        }
                        _ => return Err(ErrKind::Value),
                    },
                    "ROUND" => match flat.as_slice() {
                        [x, d] => {
                            let p = 10f64.powi(num(d)? as i32);
                            Value::Num((num(x)? * p).round() / p)
                        }
                        _ => return Err(ErrKind::Value),
                    },
                    "LEN" => Value::Num(flat.first().map_or(0, |v| v.to_string().chars().count()) as f64),
                    "UPPER" => Value::Text(flat.first().map(|v| v.to_string().to_uppercase()).unwrap_or_default()),
                    "IFERROR" => match raw.as_slice() {
                        [Ok(v), _] => v.clone(),
                        [Err(_), alt] => alt.clone()?,
                        _ => return Err(ErrKind::Value),
                    },
                    n if n.starts_with("#NAME") => return Err(ErrKind::Name),
                    _ => return Err(ErrKind::Name),
                }
            }
        })
    }

    fn render(&self, cols: u32, rows: u32) -> String {
        let w = 11;
        let mut s = format!("{:>4}", "");
        for c in 0..cols {
            let name = CellRef { col: c, row: 0 }.to_string();
            s += &format!("|{:^w$}", name.trim_end_matches('1'), w = w);
        }
        s.push('\n');
        for r in 0..rows {
            s += &format!("{:>4}", r + 1);
            for c in 0..cols {
                let v = self.get(CellRef { col: c, row: r });
                let txt = v.to_string();
                let txt: String = txt.chars().take(w).collect();
                match v {
                    Value::Num(_) => s += &format!("|{:>w$}", txt, w = w),
                    _ => s += &format!("|{:<w$}", txt, w = w),
                }
            }
            s.push('\n');
        }
        s
    }
}

fn main() {
    for r in ["A1", "Z9", "AA10", "$B$7", "AZ1", "ZZ99", "a1", "1A", "A0", "AAA1"] {
        match CellRef::parse(r) {
            Some(c) => println!("{:>5} -> ({}, {}) -> {}", r, c.col, c.row, c),
            None => println!("{:>5} -> invalid", r),
        }
    }
    let mut sh = Sheet::default();
    let inputs = [
        ("A1", "Item"), ("B1", "Qty"), ("C1", "Price"), ("D1", "Total"),
        ("A2", "Widget"), ("B2", "4"), ("C2", "2.5"), ("D2", "=B2*C2"),
        ("A3", "Gadget"), ("B3", "10"), ("C3", "1.25"), ("D3", "=B3*C3"),
        ("A4", "Doohickey"), ("B4", "1"), ("C4", "99.99"), ("D4", "=B4*C4"),
        ("A5", "Total"), ("B5", "=SUM(B2:B4)"), ("D5", "=SUM(D2:D4)"),
        ("A6", "Avg price"), ("C6", "=ROUND(AVERAGE(C2:C4), 2)"),
        ("A7", "Big?"), ("D7", "=IF(D5>100, \"big\" & \"!\", \"small\")"),
        ("A8", "Tax"), ("C8", "8%"), ("D8", "=D5*C8"),
        ("A9", "Grand"), ("D9", "=D5+D8"),
    ];
    for (c, v) in inputs {
        sh.set(c, v);
    }
    print!("{}", sh.render(4, 9));
    println!("evals so far {}", sh.evals);
    let changed = sh.set("B2", "40");
    println!("set B2=40 recalculated {:?}", changed.iter().map(|c| c.to_string()).collect::<Vec<_>>());
    println!("D9 = {} D7 = {}", sh.get(CellRef::parse("D9").unwrap()), sh.get(CellRef::parse("D7").unwrap()));
    sh.set("C8", "=1/(B4-1)");
    println!("div0 chain: C8={} D8={} D9={}", sh.get(CellRef::parse("C8").unwrap()), sh.get(CellRef::parse("D8").unwrap()), sh.get(CellRef::parse("D9").unwrap()));
    sh.set("E1", "=IFERROR(D9, -1)");
    println!("IFERROR -> {}", sh.get(CellRef::parse("E1").unwrap()));
    sh.set("C8", "0.1");
    println!("fixed: D9={} E1={}", sh.get(CellRef::parse("D9").unwrap()), sh.get(CellRef::parse("E1").unwrap()));
    // cycles
    sh.set("F1", "=F2+1");
    sh.set("F2", "=F3+1");
    let cyc = sh.set("F3", "=F1+1");
    println!("cycle cells {:?}: {} {} {}", cyc.iter().map(|c| c.to_string()).collect::<Vec<_>>(), sh.get(CellRef::parse("F1").unwrap()), sh.get(CellRef::parse("F2").unwrap()), sh.get(CellRef::parse("F3").unwrap()));
    sh.set("F3", "5");
    println!("cycle broken: F1={} F2={} F3={}", sh.get(CellRef::parse("F1").unwrap()), sh.get(CellRef::parse("F2").unwrap()), sh.get(CellRef::parse("F3").unwrap()));
    for f in ["=1+", "=SUM(A1:", "=\"abc", "=FOO(1)", "=bar", "=2^3^2", "=-2^2", "=(1+2)*3-4/2", "=\"a\"<\"B\"", "=LEN(A4)&UPPER(A2)", "=COUNT(A1:D4)+COUNTA(A1:D4)", "=MAX(B2:B4)-MIN(C2:C4)", "=A2+1", "=IF(1,2)"] {
        sh.set("G1", f);
        let src = match sh.cells.get(&CellRef::parse("G1").unwrap()) {
            Some(Content::Formula(_, Err(e))) => format!(" ({})", e),
            _ => String::new(),
        };
        println!("{:<30} => {}{}", f, sh.get(CellRef::parse("G1").unwrap()), src);
    }
    let total_cells = sh.cells.len();
    let formulas = sh.cells.values().filter(|c| matches!(c, Content::Formula(..))).count();
    let deepest = sh.fdeps.iter().map(|(c, d)| (d.len(), *c)).max();
    println!("cells {} formulas {} widest {:?} evals {}", total_cells, formulas, deepest.map(|(n, c)| (n, c.to_string())), sh.evals);
    // a long chain to exercise incremental recalc
    let mut chain = Sheet::default();
    chain.set("A1", "1");
    for i in 2..=300 {
        chain.set(&format!("A{}", i), &format!("=A{}+{}", i - 1, i % 7));
    }
    let before = chain.evals;
    chain.set("A1", "1000");
    println!("chain A300={} recalcs {}", chain.get(CellRef::parse("A300").unwrap()), chain.evals - before);
}
