// Breadth test of Option and Result combinators.
#[derive(Debug, Clone, Default, PartialEq)]
struct Cfg { name: String, port: u16 }

fn parse_port(s: &str) -> Result<u16, String> {
    s.parse::<u16>().map_err(|e| format!("{:?}: {}", s, e))
}

fn main() {
    let a: Option<i32> = Some(4);
    let n: Option<i32> = None;
    println!("zip {:?} {:?} xor {:?} {:?} {:?}", a.zip(Some('x')), a.zip(None::<char>), a.xor(n), a.xor(Some(1)), n.xor(None));
    println!("and {:?} or {:?} or_else {:?} and_then {:?} {:?}", a.and(Some("s")), n.or(Some(9)), n.or_else(|| Some(-1)), a.and_then(|x| if x > 3 { Some(x * 2) } else { None }), Some(1).and_then(|x| if x > 3 { Some(x) } else { None }));
    println!("filter {:?} {:?} map_or {} {} map_or_else {} {}", a.filter(|x| x % 2 == 0), a.filter(|x| *x > 10), a.map_or(0, |x| x + 1), n.map_or(0, |x| x + 1), a.map_or_else(|| -1, |x| x * 3), n.map_or_else(|| -1, |x| x * 3));
    println!("ok_or {:?} {:?} ok_or_else {:?}", a.ok_or("none"), n.ok_or("none"), n.ok_or_else(|| format!("missing at {}", 7)));
    println!("unwrap_or {} unwrap_or_else {} unwrap_or_default {} {:?}", n.unwrap_or(5), n.unwrap_or_else(|| 6), n.unwrap_or_default(), None::<Cfg>.unwrap_or_default());
    println!("is_some_and {} {} is_none_or {} {} is_none {}", a.is_some_and(|x| x > 3), n.is_some_and(|x| x > 3), n.is_none_or(|x| x > 100), a.is_none_or(|x| x > 100), n.is_none());
    let s: Option<String> = Some("hello".into());
    println!("as_deref {:?} as_ref map len {:?} cloned {:?}", s.as_deref(), s.as_ref().map(|x| x.len()), s.as_ref().cloned());
    let r = 5;
    let rr: Option<&i32> = Some(&r);
    println!("copied {:?} cloned {:?}", rr.copied(), rr.cloned());
    let nested: Option<Option<i32>> = Some(Some(3));
    println!("flatten {:?} {:?}", nested.flatten(), Some(None::<i32>).flatten());
    let t: Option<Result<i32, String>> = Some(Ok(2));
    let t2: Option<Result<i32, String>> = Some(Err("bad".into()));
    let t3: Option<Result<i32, String>> = None;
    println!("transpose {:?} {:?} {:?}", t.transpose(), t2.transpose(), t3.transpose());
    let rt: Result<Option<i32>, String> = Ok(Some(1));
    println!("result transpose {:?}", rt.transpose());
    let mut seen = vec![];
    let ins = a.inspect(|x| seen.push(*x)).map(|x| x + 1);
    println!("inspect {:?} seen {:?}", ins, seen);
    let mut o = Some(10);
    println!("take {:?} -> {:?}", o.take(), o);
    let mut o2 = Some(1);
    println!("replace {:?} -> {:?}", o2.replace(2), o2);
    let mut o3: Option<Vec<i32>> = None;
    o3.get_or_insert_with(Vec::new).push(5);
    o3.get_or_insert_with(Vec::new).push(6);
    println!("get_or_insert_with {:?}", o3);
    let mut o4 = None;
    *o4.get_or_insert(0) += 7;
    println!("get_or_insert {:?} insert {:?}", o4, { let mut z = Some(1); *z.insert(9) += 1; z });
    let mut o5 = Some(3);
    if let Some(x) = o5.as_mut() {
        *x *= 100;
    }
    println!("as_mut {:?}", o5);
    let t1 = o5.take_if(|x| *x > 1000);
    let t2 = o5.take_if(|x| *x == 300);
    println!("take_if {:?} {:?} -> {:?}", t1, t2, o5);
    println!("unzip {:?} {:?}", Some((1, 'a')).unzip(), None::<(i32, char)>.unzip());
    println!("iter sum {} chain {:?}", a.iter().sum::<i32>(), a.into_iter().chain(n).chain(Some(8)).collect::<Vec<_>>());
    println!("ordering: {} {} {:?}", None < Some(0), Some(1) < Some(2), [Some(3), None, Some(1)].iter().max());
    println!("Option<Option> eq {}", Some(None::<i32>) != None);

    // Result
    let good: Result<i32, String> = Ok(3);
    let bad: Result<i32, String> = Err("boom".into());
    println!("ok {:?} err {:?} {:?} {:?}", good.clone().ok(), good.clone().err(), bad.clone().ok(), bad.clone().err());
    println!("map {:?} map_err {:?} map_or {} map_or_else {}", good.clone().map(|x| x * 2), bad.clone().map_err(|e| e.len()), bad.clone().map_or(0, |x| x), bad.clone().map_or_else(|e| e.len() as i32, |x| x));
    println!("and_then {:?} {:?} or_else {:?}", good.clone().and_then(|x| if x > 5 { Ok(x) } else { Err(format!("{} too small", x)) }), bad.clone().and_then(|x| Ok::<i32, String>(x + 1)), bad.clone().or_else(|e| if e == "boom" { Ok::<i32, String>(0) } else { Err(e) }));
    println!("and {:?} or {:?}", good.clone().and(Ok::<&str, String>("next")), bad.clone().or(Ok::<i32, i32>(42)));
    println!("unwrap_or {} unwrap_or_else {} unwrap_or_default {}", bad.clone().unwrap_or(-1), bad.clone().unwrap_or_else(|e| e.len() as i32), bad.clone().unwrap_or_default());
    println!("is_ok_and {} is_err_and {} {}", good.as_ref().is_ok_and(|x| *x == 3), bad.as_ref().is_err_and(|e| e.starts_with('b')), good.as_ref().is_err_and(|_| true));
    println!("as_deref {:?} copied {:?} cloned {:?}", Ok::<String, ()>("x".into()).as_deref(), Ok::<&i32, ()>(&5).copied(), Ok::<&String, ()>(&"y".to_string()).cloned());
    let mut log = vec![];
    let _ = good.as_ref().inspect(|x| log.push(format!("ok {}", x))).inspect_err(|e| log.push(format!("err {}", e)));
    let _ = bad.as_ref().inspect(|x| log.push(format!("ok {}", x))).inspect_err(|e| log.push(format!("err {}", e)));
    println!("inspect log {:?}", log);
    let rr: Result<Result<i32, String>, String> = Ok(Err("inner".into()));
    println!("flatten {:?}", rr.and_then(|r| r));
    println!("iter {:?} {:?}", good.iter().collect::<Vec<_>>(), bad.iter().count());
    println!("expect_err {}", bad.clone().expect_err("should be err"));
    println!("copied ok {:?}", Some(&7).copied().ok_or(()));
    let ports: Vec<Result<u16, String>> = ["80", "x", "70000", "443"].iter().map(|s| parse_port(s)).collect();
    println!("ports {:?}", ports);
    let (oks, errs): (Vec<_>, Vec<_>) = ports.into_iter().partition(|r| r.is_ok());
    println!("oks {:?} errs {}", oks.into_iter().map(Result::unwrap).collect::<Vec<_>>(), errs.len());
    let all: Result<Vec<u16>, String> = ["1", "2"].iter().map(|s| parse_port(s)).collect();
    let first_err: Result<Vec<u16>, String> = ["1", "a", "b"].iter().map(|s| parse_port(s)).collect();
    println!("collect {:?} {:?}", all, first_err);
    let sum: Result<u32, String> = ["1", "2", "3"].iter().map(|s| parse_port(s).map(u32::from)).sum();
    println!("sum {:?}", sum);
    fn chain(s: &str) -> Result<Cfg, String> {
        let (name, port) = s.split_once(':').ok_or_else(|| format!("no colon in {:?}", s))?;
        let port = parse_port(port)?;
        if name.is_empty() {
            Err("empty name")?;
        }
        Ok(Cfg { name: name.to_string(), port })
    }
    for c in ["db:5432", "web", ":80", "x:999999", "api:8080"] {
        println!("{:<10} -> {:?}", c, chain(c));
    }
    let opt_q = || -> Option<char> {
        let first = "abc".chars().next()?;
        let w = "".chars().next()?;
        Some(if first < w { first } else { w })
    };
    println!("? on option {:?}", opt_q());
    println!("ordering Result: {} {}", Ok::<i32, i32>(5) < Err(0), Err::<i32, i32>(1) < Err(2));
    let mut res: Result<Vec<i32>, ()> = Ok(vec![]);
    if let Ok(v) = res.as_mut() {
        v.push(1);
    }
    println!("as_mut {:?} unwrap_unchecked-free {:?}", res, res.as_ref().map(|v| v.len()));
    let boxed: Result<i32, Box<dyn std::error::Error>> = "12x".parse::<i32>().map_err(|e| e.into());
    println!("boxed err: {}", boxed.unwrap_err());
}
