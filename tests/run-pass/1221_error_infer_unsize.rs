// Error types inferred late: `?` in immediately-called closures whose
// error type comes from the expected type, and boxing/borrowing closure
// parameters whose type is still a projection (`<T as FromStr>::Err`) as
// trait objects.
use std::error::Error;
use std::fmt::{self, Display};

#[derive(Debug)]
struct ConfigError {
    key: String,
    source: Option<Box<dyn Error + Send + Sync>>,
}

impl Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "bad value for key {:?}", self.key)
    }
}

impl Error for ConfigError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.source.as_ref().map(|e| e.as_ref() as &(dyn Error + 'static))
    }
}

type BoxErr = Box<dyn Error + Send + Sync + 'static>;

fn parse_kv(s: &str) -> Result<(String, i32), BoxErr> {
    let (k, v) = s.split_once('=').ok_or_else(|| format!("no '=' in {:?}", s))?;
    let v: i32 = v.trim().parse().map_err(|e| ConfigError { key: k.trim().to_string(), source: Some(Box::new(e)) })?;
    Ok((k.trim().to_string(), v))
}

fn chain(e: &(dyn Error + 'static)) -> Vec<String> {
    let mut v = vec![e.to_string()];
    let mut cur = e.source();
    while let Some(s) = cur {
        v.push(s.to_string());
        cur = s.source();
    }
    v
}

fn describe(r: Result<i32, std::num::ParseIntError>) -> String {
    r.map(|v| v.to_string()).unwrap_or_else(|e| {
        let d: &dyn Display = &e;
        format!("err: {}", d)
    })
}

fn closures_with_question_mark() {
    let v = vec!["1", "2", "x"];
    let r: Result<Vec<i32>, Box<dyn Error>> = v.iter().map(|s| Ok(s.parse::<i32>()?)).collect();
    println!("{:?}", r.map_err(|e| e.to_string()));
    let r2: Result<Vec<i32>, String> = v[..2].iter().map(|s| Ok(s.parse::<i32>().map_err(|e| e.to_string())? * 2)).collect();
    println!("{:?}", r2);
    let total: Result<i32, Box<dyn Error>> = v[..2].iter().try_fold(0, |acc, s| Ok(acc + s.parse::<i32>()?));
    println!("{:?}", total.map_err(|e| e.to_string()));
    let o = Some(4).and_then(|x: i32| { let y = x.checked_mul(2)?; Some(y + 1) });
    println!("{:?}", o);
    let sums: Vec<Result<u8, Box<dyn Error>>> = vec!["250", "10"].into_iter().map(|s| { let n: u8 = s.parse()?; Ok(n.checked_add(10).ok_or("ovf")?) }).collect();
    println!("{:?}", sums.into_iter().map(|r| r.map_err(|e| e.to_string())).collect::<Vec<_>>());
}

fn main() {
    closures_with_question_mark();
    for s in ["a = 5", "b=x", "noeq", " c = -12 "] {
        match parse_kv(s) {
            Ok(kv) => println!("{:?}", kv),
            Err(e) => println!("{:?}", chain(e.as_ref())),
        }
    }

    let r: Result<(), Box<dyn Error>> = (|| {
        let _: u8 = "300".parse()?;
        Ok(())
    })();
    println!("{}", r.unwrap_err());
    let r2: Result<u8, Box<dyn Error>> = (|| {
        let a = "7".parse::<u8>()?;
        let b: u8 = "8".parse()?;
        let c = a.checked_add(b).ok_or("overflow")?;
        Ok(c)
    })();
    println!("{:?}", r2.map_err(|e| e.to_string()));
    let r3: Result<i64, String> = (|| {
        let n: i64 = "12".parse().map_err(|e: std::num::ParseIntError| e.to_string())?;
        Ok(n * 2)
    })();
    println!("{:?}", r3);

    println!("{}", describe("5".parse()));
    println!("{}", describe("q".parse()));
    let errs: Vec<Box<dyn Error>> = vec!["1", "x", "300"]
        .into_iter()
        .filter_map(|s| s.parse::<u8>().err())
        .map(|e| Box::new(e) as Box<dyn Error>)
        .collect();
    println!("{:?}", errs.iter().map(|e| e.to_string()).collect::<Vec<_>>());
    let r: Result<u16, Box<dyn Error>> = "70000".parse().map_err(|e| Box::new(e) as Box<dyn Error>);
    println!("{:?}", r.map_err(|e| e.to_string()));
    let f: Result<f32, Box<dyn Error + Send + Sync>> = "1.5x".parse().map_err(|e| {
        let b: Box<dyn Error + Send + Sync> = Box::new(e);
        b
    });
    println!("{:?}", f.map_err(|e| e.to_string()));
    let shown: Vec<String> = vec![Ok::<i32, String>(1), Err("bad".to_string())]
        .into_iter()
        .map(|r| match r {
            Ok(v) => {
                let b: Box<dyn Display> = Box::new(v);
                b.to_string()
            }
            Err(e) => {
                let b: Box<dyn Display> = Box::new(e);
                b.to_string()
            }
        })
        .collect();
    println!("{:?}", shown);
}
