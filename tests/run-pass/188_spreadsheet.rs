use std::collections::{BTreeMap, HashSet};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
struct CellRef {
    col: u8,
    row: u32,
}

impl CellRef {
    fn parse(s: &str) -> Option<CellRef> {
        let mut chars = s.chars();
        let c = chars.next()?;
        if !c.is_ascii_uppercase() {
            return None;
        }
        let row: u32 = chars.as_str().parse().ok()?;
        Some(CellRef { col: c as u8 - b'A', row })
    }
}

impl fmt::Display for CellRef {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}{}", (b'A' + self.col) as char, self.row)
    }
}

#[derive(Debug, Clone)]
enum Formula {
    Lit(f64),
    Ref(CellRef),
    Sum(CellRef, CellRef),
    Avg(CellRef, CellRef),
    Bin(Box<Formula>, char, Box<Formula>),
}

#[derive(Debug, Clone, PartialEq)]
enum Value {
    Num(f64),
    Error(String),
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Value::Num(n) if n.fract() == 0.0 => write!(f, "{}", n),
            Value::Num(n) => write!(f, "{:.2}", n),
            Value::Error(e) => write!(f, "#{}", e),
        }
    }
}

fn parse_formula(s: &str) -> Formula {
    let s = s.trim();
    // lowest precedence: + - (scan right-to-left outside parens)
    for ops in [['+', '-'], ['*', '/']] {
        let mut depth = 0;
        for (i, ch) in s.char_indices().rev() {
            match ch {
                ')' => depth += 1,
                '(' => depth -= 1,
                c if depth == 0 && ops.contains(&c) && i > 0 => {
                    return Formula::Bin(Box::new(parse_formula(&s[..i])), c, Box::new(parse_formula(&s[i + 1..])));
                }
                _ => {}
            }
        }
    }
    if let Some(inner) = s.strip_prefix("SUM(").and_then(|r| r.strip_suffix(')')) {
        let (a, b) = inner.split_once(':').unwrap();
        return Formula::Sum(CellRef::parse(a).unwrap(), CellRef::parse(b).unwrap());
    }
    if let Some(inner) = s.strip_prefix("AVG(").and_then(|r| r.strip_suffix(')')) {
        let (a, b) = inner.split_once(':').unwrap();
        return Formula::Avg(CellRef::parse(a).unwrap(), CellRef::parse(b).unwrap());
    }
    if let Some(inner) = s.strip_prefix('(').and_then(|r| r.strip_suffix(')')) {
        return parse_formula(inner);
    }
    if let Some(r) = CellRef::parse(s) {
        return Formula::Ref(r);
    }
    Formula::Lit(s.parse().unwrap_or(f64::NAN))
}

struct Sheet {
    cells: BTreeMap<CellRef, Formula>,
}

impl Sheet {
    fn set(&mut self, at: &str, src: &str) {
        let r = CellRef::parse(at).unwrap();
        let f = match src.strip_prefix('=') {
            Some(expr) => parse_formula(expr),
            None => Formula::Lit(src.parse().unwrap()),
        };
        self.cells.insert(r, f);
    }

    fn range(a: CellRef, b: CellRef) -> Vec<CellRef> {
        let mut v = Vec::new();
        for col in a.col..=b.col {
            for row in a.row..=b.row {
                v.push(CellRef { col, row });
            }
        }
        v
    }

    fn eval_cell(&self, r: CellRef, visiting: &mut HashSet<CellRef>) -> Value {
        if !visiting.insert(r) {
            return Value::Error(format!("CYCLE@{}", r));
        }
        let v = match self.cells.get(&r) {
            None => Value::Num(0.0),
            Some(f) => self.eval(f, visiting),
        };
        visiting.remove(&r);
        v
    }

    fn eval(&self, f: &Formula, visiting: &mut HashSet<CellRef>) -> Value {
        match f {
            Formula::Lit(n) if n.is_nan() => Value::Error("PARSE".into()),
            Formula::Lit(n) => Value::Num(*n),
            Formula::Ref(r) => self.eval_cell(*r, visiting),
            Formula::Sum(a, b) | Formula::Avg(a, b) => {
                let cells = Sheet::range(*a, *b);
                let mut total = 0.0;
                for c in &cells {
                    match self.eval_cell(*c, visiting) {
                        Value::Num(n) => total += n,
                        e => return e,
                    }
                }
                if let Formula::Avg(..) = f {
                    Value::Num(total / cells.len() as f64)
                } else {
                    Value::Num(total)
                }
            }
            Formula::Bin(l, op, r) => match (self.eval(l, visiting), self.eval(r, visiting)) {
                (Value::Num(x), Value::Num(y)) => match op {
                    '+' => Value::Num(x + y),
                    '-' => Value::Num(x - y),
                    '*' => Value::Num(x * y),
                    '/' if y == 0.0 => Value::Error("DIV0".into()),
                    '/' => Value::Num(x / y),
                    _ => Value::Error("OP".into()),
                },
                (e @ Value::Error(_), _) | (_, e @ Value::Error(_)) => e,
            },
        }
    }

    fn print(&self) {
        let max_row = self.cells.keys().map(|r| r.row).max().unwrap_or(0);
        let max_col = self.cells.keys().map(|r| r.col).max().unwrap_or(0);
        print!("    ");
        for c in 0..=max_col {
            print!("{:>12}", (b'A' + c) as char);
        }
        println!();
        for row in 1..=max_row {
            print!("{:>3} ", row);
            for col in 0..=max_col {
                let v = self.eval_cell(CellRef { col, row }, &mut HashSet::new());
                print!("{:>12}", v.to_string());
            }
            println!();
        }
    }
}

fn main() {
    let mut s = Sheet { cells: BTreeMap::new() };
    s.set("A1", "10");
    s.set("A2", "20");
    s.set("A3", "30.5");
    s.set("A4", "=SUM(A1:A3)");
    s.set("B1", "=A1*2");
    s.set("B2", "=B1+A2/4");
    s.set("B3", "=(A1+A2)*(A3-0.5)");
    s.set("B4", "=AVG(B1:B3)");
    s.set("C1", "=C2+1");
    s.set("C2", "=C1+1");
    s.set("C3", "=A1/(A2-20)");
    s.set("C4", "=SUM(A1:B2)-A4");
    s.set("D1", "=abc");
    s.set("D2", "=D1+5");
    s.print();
    println!("--- update A1 = 100");
    s.set("A1", "100");
    s.print();
    let r = CellRef::parse("B4").unwrap();
    println!("{} = {}", r, s.eval_cell(r, &mut HashSet::new()));
}
