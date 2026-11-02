// Heavy use of Option/Result combinators, as found in library code.
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
struct User {
    id: u32,
    name: String,
    email: Option<String>,
    manager: Option<u32>,
}

fn users() -> HashMap<u32, User> {
    let list = vec![
        User { id: 1, name: "ada".into(), email: Some("ada@x.org".into()), manager: None },
        User { id: 2, name: "bob".into(), email: None, manager: Some(1) },
        User { id: 3, name: "cy".into(), email: Some("cy@y.com".into()), manager: Some(2) },
        User { id: 4, name: "dee".into(), email: Some("bad-email".into()), manager: Some(9) },
    ];
    list.into_iter().map(|u| (u.id, u)).collect()
}

fn email_domain(u: &User) -> Option<&str> {
    u.email.as_deref().and_then(|e| e.split_once('@')).map(|(_, d)| d)
}

fn manager_name(db: &HashMap<u32, User>, id: u32) -> Result<String, String> {
    db.get(&id)
        .ok_or(format!("no user {}", id))?
        .manager
        .ok_or_else(|| format!("user {} has no manager", id))
        .and_then(|m| db.get(&m).ok_or(format!("manager {} missing", m)))
        .map(|m| m.name.to_uppercase())
}

fn parse_kv(s: &str) -> Result<(String, i32), String> {
    let (k, v) = s.split_once('=').ok_or_else(|| format!("no '=' in {:?}", s))?;
    let v = v.trim().parse::<i32>().map_err(|e| format!("{}: {}", k, e))?;
    Some(k.trim()).filter(|k| !k.is_empty()).map(|k| (k.to_string(), v)).ok_or_else(|| "empty key".to_string())
}

fn main() {
    let db = users();
    let mut ids: Vec<u32> = db.keys().copied().collect();
    ids.sort();
    for id in &ids {
        let u = &db[id];
        println!(
            "{} {:<4} domain={:<10} mgr={:<30} email_len={}",
            id,
            u.name,
            email_domain(u).unwrap_or("-"),
            format!("{:?}", manager_name(&db, *id)),
            u.email.as_ref().map_or(0, |e| e.len())
        );
    }
    println!("{:?}", manager_name(&db, 42));

    // Option combinators
    let a: Option<i32> = Some(4);
    let b: Option<i32> = None;
    println!("{:?} {:?}", a.map(|x| x * 2), b.map(|x| x * 2));
    println!("{:?} {:?}", a.filter(|x| x % 2 == 1), a.xor(b));
    println!("{:?} {:?}", a.zip(Some('c')), a.zip(b));
    println!("{:?} {:?}", a.or(Some(9)), b.or_else(|| Some(10)));
    println!("{} {}", a.map_or(-1, |x| x + 1), b.map_or_else(|| -2, |x| x + 1));
    println!("{} {}", a.is_some_and(|x| x > 3), b.is_none_or(|x| x > 3));
    println!("{:?} {:?}", a.and(Some("yes")), b.and(Some("yes")));
    println!("{:?}", Some(Some(3)).flatten().into_iter().chain(None).collect::<Vec<i32>>());
    let mut c = Some(5);
    println!("take {:?} then {:?}", c.take(), c);
    println!("insert {:?}", *c.get_or_insert(7));
    println!("replace {:?} now {:?}", c.replace(8), c);
    let mut d: Option<Vec<i32>> = None;
    d.get_or_insert_with(Vec::new).push(1);
    d.as_mut().map(|v| v.push(2));
    println!("{:?} unzip {:?}", d, Some((1, "a")).unzip());

    // Result combinators
    let r1: Result<i32, String> = Ok(10);
    let r2: Result<i32, String> = Err("boom".into());
    println!("{:?} {:?}", r1.as_ref().map(|x| x + 1), r2.as_ref().map_err(|e| e.len()));
    println!("{:?} {:?}", r1.clone().ok(), r2.clone().err());
    println!("{} {}", r1.clone().unwrap_or_default(), r2.clone().unwrap_or_else(|e| e.len() as i32));
    println!("{:?}", r2.clone().or_else(|e| if e == "boom" { Ok::<i32, String>(0) } else { Err(e) }));
    println!("{:?}", r1.clone().and_then(|x| if x > 5 { Err(format!("{} too big", x)) } else { Ok(x) }));
    println!("{} {}", r1.as_ref().is_ok_and(|x| *x == 10), r2.as_ref().is_err_and(|e| e.starts_with('b')));
    println!("{}", r1.as_ref().map_or_else(|e| e.clone(), |v| format!("v={}", v)));
    let nested: Result<Option<i32>, String> = Ok(Some(3));
    println!("{:?}", nested.transpose());
    let opt_res: Option<Result<i32, String>> = Some(Err("x".into()));
    println!("{:?}", opt_res.transpose());
    println!("{:?}", r1.iter().chain(r2.iter()).collect::<Vec<_>>());

    for s in ["a = 5", "b=x", "noeq", " = 3", "long_key=  -12 "] {
        println!("{:<16} => {:?}", format!("{:?}", s), parse_kv(s));
    }
    let (oks, errs): (Vec<_>, Vec<_>) = ["x=1", "y", "z=3"].iter().map(|s| parse_kv(s)).partition(|r| r.is_ok());
    println!("{} ok, {} err; values {:?}", oks.len(), errs.len(), oks.into_iter().map(Result::unwrap).collect::<Vec<_>>());
    let firsterr = ["x=1", "y", "z"].iter().map(|s| parse_kv(s)).find_map(Result::err);
    println!("{:?}", firsterr);
}
