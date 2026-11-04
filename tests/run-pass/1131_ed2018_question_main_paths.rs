// edition:2018
// Edition 2018: `crate::`/`self::`/`super::` paths, extern-crate-less std
// paths, `dyn Trait`, `impl Trait`, `?` with Box<dyn Error> in main, and a
// failing main that returns Err (exit code 1, error printed to stderr).
use std::error::Error;
use std::fmt;

mod parsing {
    use crate::model::{Item, Unit};
    use std::str::FromStr;

    #[derive(Debug)]
    pub struct ParseItemError(pub String);

    impl std::fmt::Display for ParseItemError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "cannot parse item: {}", self.0)
        }
    }
    impl std::error::Error for ParseItemError {}

    impl FromStr for Unit {
        type Err = ParseItemError;
        fn from_str(s: &str) -> Result<Self, Self::Err> {
            match s {
                "kg" => Ok(Unit::Kg),
                "pcs" => Ok(Unit::Pieces),
                "l" => Ok(Unit::Litre),
                other => Err(ParseItemError(format!("unknown unit {:?}", other))),
            }
        }
    }

    pub fn parse_line(line: &str) -> Result<Item, Box<dyn std::error::Error>> {
        let mut parts = line.split(',').map(str::trim);
        let name = parts.next().filter(|s| !s.is_empty()).ok_or_else(|| ParseItemError("missing name".into()))?;
        let qty: f64 = parts.next().ok_or_else(|| ParseItemError("missing qty".into()))?.parse()?;
        let unit: Unit = parts.next().unwrap_or("pcs").parse()?;
        Ok(Item { name: name.to_string(), qty, unit })
    }

    pub mod strict {
        pub fn parse_all(text: &str) -> Result<Vec<super::Item>, Box<dyn std::error::Error>> {
            text.lines().filter(|l| !l.trim().is_empty()).map(super::parse_line).collect()
        }
    }
}

mod model {
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub enum Unit { Kg, Pieces, Litre }

    #[derive(Debug, Clone)]
    pub struct Item { pub name: String, pub qty: f64, pub unit: Unit }

    impl Item {
        pub fn describe(&self) -> String {
            let u = match self.unit { Unit::Kg => "kg", Unit::Pieces => "pcs", Unit::Litre => "l" };
            format!("{} {} {}", self.qty, u, self.name)
        }
    }
}

use self::model::Unit;
use crate::parsing::strict;

#[derive(Debug)]
struct Fatal { code: u8 }
impl fmt::Display for Fatal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { write!(f, "fatal error code {}", self.code) }
}
impl Error for Fatal {}

fn shout(items: &[model::Item]) -> impl Iterator<Item = String> + '_ {
    items.iter().map(|i| i.describe().to_uppercase())
}

fn report(e: &dyn Error) -> String {
    let mut s = e.to_string();
    let mut cur = e.source();
    while let Some(c) = cur {
        s.push_str(&format!(" <- {}", c));
        cur = c.source();
    }
    s
}

fn main() -> Result<(), Box<dyn Error>> {
    let text = "apples, 2.5, kg\n  bread, 1\n\nmilk, 2, l\n";
    let items = strict::parse_all(text)?;
    for line in shout(&items) {
        println!("{}", line);
    }
    let kg: f64 = items.iter().filter(|i| i.unit == Unit::Kg).map(|i| i.qty).sum();
    println!("total kg {}", kg);
    for bad in &["eggs, many", ", 3", "fish, 1, oz", "rice"] {
        match parsing::parse_line(bad) {
            Ok(i) => println!("{:?} ok {:?}", bad, i),
            Err(e) => println!("{:?} -> {}", bad, report(e.as_ref())),
        }
    }
    let n: i32 = "12".parse()?;
    println!("n = {}", n);
    let boxed: Box<dyn Error + Send + Sync> = From::from("plain string error");
    println!("{}", boxed);
    if n > 10 {
        Err(Box::new(Fatal { code: 3 }))?;
    }
    println!("unreachable");
    Ok(())
}
