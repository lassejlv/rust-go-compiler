// let-else with various refutable patterns (enum, slice, tuple, struct), matches! with guards,
// let-else in loops with continue/break, and if-let ... else if let chains.
use std::collections::HashMap;

#[derive(Debug)]
enum Cmd {
    Push(i64),
    Pop,
    Add,
    Mul,
    Dup,
    Jz(usize),
}

fn parse_line(line: &str) -> Result<Cmd, String> {
    let mut parts = line.split_whitespace();
    let Some(op) = parts.next() else {
        return Err("empty line".into());
    };
    let cmd = match op {
        "push" => {
            let Some(arg) = parts.next() else {
                return Err("push needs arg".into());
            };
            let Ok(n) = arg.parse::<i64>() else {
                return Err(format!("bad number {:?}", arg));
            };
            Cmd::Push(n)
        }
        "jz" => {
            let Some(Ok(t)) = parts.next().map(|s| s.parse::<usize>()) else {
                return Err("jz needs target".into());
            };
            Cmd::Jz(t)
        }
        "pop" => Cmd::Pop,
        "add" => Cmd::Add,
        "mul" => Cmd::Mul,
        "dup" => Cmd::Dup,
        other => return Err(format!("unknown op {}", other)),
    };
    if let Some(extra) = parts.next() {
        return Err(format!("trailing {}", extra));
    }
    Ok(cmd)
}

struct Config {
    entries: HashMap<String, String>,
}

impl Config {
    fn port(&self) -> Result<u16, String> {
        let Some(raw) = self.entries.get("port") else {
            return Err("no port".into());
        };
        let Ok(p) = raw.parse::<u16>() else {
            return Err(format!("port {:?} invalid", raw));
        };
        Ok(p)
    }
    fn host_parts(&self) -> Option<(String, String)> {
        let host = self.entries.get("host")?;
        let [name, domain] = host.splitn(2, '.').collect::<Vec<_>>()[..] else {
            return None;
        };
        Some((name.to_string(), domain.to_string()))
    }
}

fn main() {
    let lines = ["push 3", "push x", "", "pop now", "jz 4", "jz", "frob", "dup", "mul", "push -12"];
    let mut ok = Vec::new();
    for l in lines {
        match parse_line(l) {
            Ok(c) => ok.push(c),
            Err(e) => println!("error: {}", e),
        }
    }
    println!("parsed {:?}", ok);
    let arith = ok.iter().filter(|c| matches!(c, Cmd::Add | Cmd::Mul | Cmd::Dup)).count();
    let neg_push = ok.iter().any(|c| matches!(c, Cmd::Push(n) if *n < 0));
    let jumps = ok.iter().filter(|c| matches!(**c, Cmd::Jz(t) if t > 2)).count();
    println!("arith={} neg_push={} jumps={} pop={}", arith, neg_push, jumps, ok.iter().any(|c| matches!(c, Cmd::Pop)));

    let mut cfg = Config { entries: HashMap::new() };
    println!("{:?} {:?}", cfg.port(), cfg.host_parts());
    cfg.entries.insert("port".into(), "99999".into());
    cfg.entries.insert("host".into(), "localhost".into());
    println!("{:?} {:?}", cfg.port(), cfg.host_parts());
    cfg.entries.insert("port".into(), "8080".into());
    cfg.entries.insert("host".into(), "www.example.com".into());
    println!("{:?} {:?}", cfg.port(), cfg.host_parts());

    // let-else in loop with continue / break
    let records = ["alice:30", "bob", "carol:x", "dave:41", "STOP", "eve:22"];
    let mut ages = Vec::new();
    for r in records {
        if r == "STOP" {
            break;
        }
        let Some((name, age)) = r.split_once(':') else {
            println!("skip {:?} (no colon)", r);
            continue;
        };
        let Ok(age) = age.parse::<u8>() else {
            println!("skip {:?} (bad age)", name);
            continue;
        };
        ages.push((name, age));
    }
    println!("{:?}", ages);

    // let-else with struct + tuple patterns
    #[derive(Debug)]
    struct Pt {
        x: i32,
        y: i32,
    }
    let pts = [Pt { x: 0, y: 5 }, Pt { x: 3, y: 4 }];
    for p in &pts {
        let Pt { x: 0, y } = p else {
            println!("not on axis: {:?}", p);
            continue;
        };
        println!("on y axis at {}", y);
    }
    let v: Vec<i32> = vec![9, 8, 7];
    let [a, .., c] = v.as_slice() else { panic!("too short") };
    println!("a={} c={}", a, c);

    // if-let else-if-let chain
    let inputs: [Result<Option<i32>, String>; 4] = [Ok(Some(1)), Ok(None), Err("bad".into()), Ok(Some(-2))];
    for i in &inputs {
        if let Ok(Some(n)) = i {
            if *n > 0 {
                println!("positive {}", n);
            } else {
                println!("non-positive {}", n);
            }
        } else if let Ok(None) = i {
            println!("nothing");
        } else if let Err(e) = i {
            println!("err {}", e);
        }
    }
}
