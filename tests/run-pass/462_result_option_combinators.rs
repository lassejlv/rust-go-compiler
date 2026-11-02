// Option/Result combinator coverage: and_then/or_else/map_or/map_or_else/ok_or_else/filter/xor/zip/
// unzip/take_if/is_some_and/is_none_or/flatten/transpose/copied/cloned/as_deref/unwrap_or_default/
// inspect/iter, Result::and/or/ok/err/unwrap_or_else/map_err/is_ok_and/iter/flatten, and ? on Option.
fn first_even_sq(v: &[i32]) -> Option<i32> {
    let x = v.iter().find(|x| *x % 2 == 0)?;
    x.checked_mul(*x)
}

fn div(a: i32, b: i32) -> Result<i32, String> {
    if b == 0 { Err(format!("div {} by zero", a)) } else { Ok(a / b) }
}

fn main() {
    let some: Option<i32> = Some(4);
    let none: Option<i32> = None;
    println!("{:?} {:?}", some.map(|x| x * 2), none.map(|x| x * 2));
    println!("{:?} {:?}", some.and_then(|x| if x > 3 { Some(x - 3) } else { None }), some.and_then(|_| None::<i32>));
    println!("{:?} {:?}", none.or(Some(9)), none.or_else(|| Some(10)));
    println!("{} {}", some.map_or(0, |x| x + 1), none.map_or_else(|| -1, |x| x + 1));
    println!("{:?} {:?}", some.ok_or("none"), none.ok_or_else(|| format!("missing {}", 1)));
    println!("{:?} {:?}", some.filter(|x| x % 2 == 1), some.filter(|x| x % 2 == 0));
    println!("{:?} {:?} {:?}", some.xor(none), some.xor(Some(1)), none.xor(none));
    println!("{:?} {:?}", some.zip(Some('a')), some.zip(None::<char>));
    let (a, b): (Option<i32>, Option<&str>) = Some((1, "one")).unzip();
    println!("{:?} {:?}", a, b);
    println!("{} {} {}", some.is_some_and(|x| x > 3), none.is_some_and(|x| x > 3), none.is_none_or(|x| x > 100));
    println!("{:?} {:?}", Some(Some(3)).flatten(), Some(None::<i32>).flatten());
    let t: Option<Result<i32, String>> = Some(Ok(5));
    let tr: Result<Option<i32>, String> = t.transpose();
    println!("{:?} {:?}", tr, Ok::<Option<i32>, String>(None).transpose());
    let s = Some(String::from("text"));
    println!("{:?} {:?} {}", s.as_deref(), s.as_ref().map(|s| s.len()), None::<String>.unwrap_or_default().len());
    let r = 5;
    let rr: Option<&i32> = Some(&r);
    println!("{:?} {:?}", rr.copied(), rr.cloned().map(|x| x + 1));
    let mut slot = Some(10);
    let taken = slot.take_if(|v| *v > 5);
    println!("{:?} {:?}", taken, slot);
    let mut slot2 = Some(1);
    println!("{:?} {:?}", slot2.take_if(|v| *v > 5), slot2);
    let mut opt = Some(3);
    if let Some(x) = opt.as_mut() {
        *x += 1;
    }
    let inserted = *opt.insert(100) + 1;
    println!("{:?} {}", opt, inserted);
    let mut seen = vec![];
    let _ = some.inspect(|x| seen.push(*x)).map(|x| x + 1);
    println!("{:?} iter={:?}", seen, some.iter().chain(none.iter()).collect::<Vec<_>>());
    println!("{:?} {:?}", first_even_sq(&[1, 3, 6, 8]), first_even_sq(&[1, 3]));
    println!("{:?}", first_even_sq(&[1, 65536]));

    println!("results:");
    let ok: Result<i32, String> = Ok(10);
    let err: Result<i32, String> = Err("bad".into());
    println!("{:?} {:?}", ok.as_ref().map(|x| x * 2), err.as_ref().map_err(|e| e.len()));
    println!("{:?} {:?}", ok.clone().and_then(|x| div(x, 0)), ok.clone().and_then(|x| div(x, 3)));
    println!("{:?} {:?}", err.clone().or_else(|e| if e == "bad" { Ok::<i32, String>(0) } else { Err(e) }), err.clone().or(Ok::<i32, ()>(1)));
    println!("{:?} {:?}", ok.clone().and(Err::<u8, String>("x".into())), err.clone().and(Ok::<u8, String>(1)));
    println!("{:?} {:?} {:?} {:?}", ok.clone().ok(), ok.clone().err(), err.clone().ok(), err.clone().err());
    println!("{} {}", err.clone().unwrap_or_else(|e| e.len() as i32), err.clone().unwrap_or_default());
    println!("{} {}", ok.clone().map_or(0, |x| x * 3), err.clone().map_or_else(|e| e.len() as i32 * -1, |x| x));
    println!("{} {}", ok.as_ref().is_ok_and(|x| *x == 10), err.as_ref().is_err_and(|e| e.starts_with('b')));
    println!("{:?} {:?}", ok.iter().next(), err.iter().next());
    let nested: Result<Result<i32, String>, String> = Ok(Err("inner".into()));
    println!("{:?}", nested.and_then(|r| r));
    let copied: Result<i32, ()> = Ok::<&i32, ()>(&7).copied();
    println!("{:?}", copied);
    let divs: Vec<Result<i32, String>> = [(10, 2), (1, 0), (9, 3)].iter().map(|&(a, b)| div(a, b)).collect();
    let first_err = divs.iter().find_map(|r| r.as_ref().err());
    let oks: i32 = divs.iter().flatten().sum();
    println!("{:?} {}", first_err, oks);
    let all: Result<Vec<i32>, String> = divs.into_iter().collect();
    println!("{:?}", all);
    let chained = div(100, 5).and_then(|x| div(x, 2)).and_then(|x| div(x, 0)).map_err(|e| e.to_uppercase()).unwrap_or(-1);
    println!("{}", chained);
    let msg = match div(7, 0) {
        Ok(v) => v.to_string(),
        Err(ref e) if e.contains("zero") => "zero!".to_string(),
        Err(e) => e,
    };
    println!("{}", msg);
}
