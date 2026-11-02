// A CSV reader/writer with RFC 4180 quoting, embedded newlines, configurable delimiter, typed
// row deserialization via a FromRecord trait, and error positions.
use std::fmt;

#[derive(Debug, PartialEq)]
enum CsvError {
    UnterminatedQuote { line: usize },
    QuoteInField { line: usize, col: usize },
    FieldCount { line: usize, expected: usize, got: usize },
    Parse { line: usize, field: &'static str, msg: String },
}

impl fmt::Display for CsvError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            CsvError::UnterminatedQuote { line } => write!(f, "line {}: unterminated quote", line),
            CsvError::QuoteInField { line, col } => write!(f, "line {}:{}: stray quote", line, col),
            CsvError::FieldCount { line, expected, got } => write!(f, "line {}: expected {} fields, got {}", line, expected, got),
            CsvError::Parse { line, field, msg } => write!(f, "line {}: field `{}`: {}", line, field, msg),
        }
    }
}

struct Reader<'a> {
    src: &'a str,
    delim: char,
}

impl<'a> Reader<'a> {
    fn records(&self) -> Result<Vec<(usize, Vec<String>)>, CsvError> {
        let mut out = vec![];
        let mut field = String::new();
        let mut row: Vec<String> = vec![];
        let mut in_quotes = false;
        let mut line = 1;
        let mut row_line = 1;
        let mut col = 0;
        let mut chars = self.src.chars().peekable();
        let mut field_started_quoted = false;
        while let Some(c) = chars.next() {
            col += 1;
            if in_quotes {
                match c {
                    '"' if chars.peek() == Some(&'"') => {
                        chars.next();
                        field.push('"');
                    }
                    '"' => in_quotes = false,
                    '\n' => {
                        line += 1;
                        col = 0;
                        field.push('\n');
                    }
                    c => field.push(c),
                }
                continue;
            }
            match c {
                '"' if field.is_empty() && !field_started_quoted => {
                    in_quotes = true;
                    field_started_quoted = true;
                }
                '"' => return Err(CsvError::QuoteInField { line, col }),
                c if c == self.delim => {
                    row.push(std::mem::take(&mut field));
                    field_started_quoted = false;
                }
                '\r' => {}
                '\n' => {
                    row.push(std::mem::take(&mut field));
                    field_started_quoted = false;
                    if !(row.len() == 1 && row[0].is_empty()) {
                        out.push((row_line, std::mem::take(&mut row)));
                    } else {
                        row.clear();
                    }
                    line += 1;
                    row_line = line;
                    col = 0;
                }
                c => field.push(c),
            }
        }
        if in_quotes {
            return Err(CsvError::UnterminatedQuote { line: row_line });
        }
        if !field.is_empty() || !row.is_empty() || field_started_quoted {
            row.push(field);
            out.push((row_line, row));
        }
        Ok(out)
    }
}

fn write_field(f: &str, delim: char) -> String {
    if f.contains(delim) || f.contains('"') || f.contains('\n') || f.starts_with(' ') || f.ends_with(' ') {
        format!("\"{}\"", f.replace('"', "\"\""))
    } else {
        f.to_string()
    }
}

fn write_row(fields: &[String], delim: char) -> String {
    fields.iter().map(|f| write_field(f, delim)).collect::<Vec<_>>().join(&delim.to_string())
}

trait FromRecord: Sized {
    const FIELDS: &'static [&'static str];
    fn from_record(line: usize, r: &[String]) -> Result<Self, CsvError>;
}

#[derive(Debug)]
struct Item {
    sku: String,
    qty: u32,
    price: f64,
    note: Option<String>,
}

fn field<T: std::str::FromStr>(line: usize, name: &'static str, s: &str) -> Result<T, CsvError>
where
    T::Err: fmt::Display,
{
    s.trim().parse::<T>().map_err(|e| CsvError::Parse { line, field: name, msg: e.to_string() })
}

impl FromRecord for Item {
    const FIELDS: &'static [&'static str] = &["sku", "qty", "price", "note"];
    fn from_record(line: usize, r: &[String]) -> Result<Self, CsvError> {
        if r.len() != Self::FIELDS.len() {
            return Err(CsvError::FieldCount { line, expected: Self::FIELDS.len(), got: r.len() });
        }
        Ok(Item {
            sku: r[0].clone(),
            qty: field(line, "qty", &r[1])?,
            price: field(line, "price", &r[2])?,
            note: if r[3].is_empty() { None } else { Some(r[3].clone()) },
        })
    }
}

fn load<T: FromRecord>(src: &str) -> (Vec<T>, Vec<CsvError>) {
    let recs = match (Reader { src, delim: ',' }).records() {
        Ok(r) => r,
        Err(e) => return (vec![], vec![e]),
    };
    let mut ok = vec![];
    let mut errs = vec![];
    for (line, r) in recs.iter().skip(1) {
        match T::from_record(*line, r) {
            Ok(v) => ok.push(v),
            Err(e) => errs.push(e),
        }
    }
    (ok, errs)
}

fn main() {
    let data = "sku,qty,price,note\n\
                A-1,3,9.99,\n\
                B-2,10,0.5,\"bulk, discounted\"\n\
                C-3,1,100,\"says \"\"hi\"\"\"\n\
                D-4,x,1.0,bad qty\n\
                E-5,2,2.5,\"multi\nline note\"\n\
                \n\
                F-6,1\n\
                G-7,4,1e2,last";
    let (items, errs) = load::<Item>(data);
    for it in &items {
        println!("{:?}", it);
    }
    for e in &errs {
        println!("error: {}", e);
    }
    let total: f64 = items.iter().map(|i| i.qty as f64 * i.price).sum();
    println!("total value {:.2}", total);

    let rows: Vec<Vec<String>> = vec![
        vec!["plain".into(), "with,comma".into(), "with \"quote\"".into()],
        vec![" padded ".into(), "multi\nline".into(), "".into()],
    ];
    let mut out = String::new();
    for r in &rows {
        out.push_str(&write_row(r, ','));
        out.push('\n');
    }
    print!("{}", out);
    let back = Reader { src: &out, delim: ',' }.records().unwrap();
    println!("roundtrip ok: {}", back.iter().map(|(_, r)| r.clone()).collect::<Vec<_>>() == rows);

    let tsv = Reader { src: "a\tb\tc\n1\t\"x\ty\"\t3", delim: '\t' }.records().unwrap();
    println!("{:?}", tsv);
    for bad in ["a,\"b\nc", "a,b\"c\",d", "\"ok\",\"\""] {
        println!("{:?} -> {:?}", bad, Reader { src: bad, delim: ',' }.records());
    }
}
