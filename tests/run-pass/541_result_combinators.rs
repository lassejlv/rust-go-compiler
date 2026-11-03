// Result/Option combinators: and_then, or_else, map_err, ok_or_else, transpose, flatten, zip, xor, filter, take, replace, get_or_insert_with, is_some_and, inspect
fn half(x: i32) -> Result<i32, String> {
    if x % 2 == 0 { Ok(x / 2) } else { Err(format!("{} is odd", x)) }
}
fn non_neg(x: i32) -> Option<i32> {
    if x >= 0 { Some(x) } else { None }
}

fn main() {
    println!("{:?}", half(20).and_then(half).and_then(half));
    println!("{:?}", half(12).and_then(half).and_then(half));
    println!("{:?}", half(3).or_else(|e| if e.contains("3") { Ok(-1) } else { Err(e) }));
    println!("{:?}", half(5).or_else(|_| half(6)).map(|v| v * 100));
    println!("{:?}", half(7).map_err(|e| e.len()));
    println!("{:?}", half(8).map_or(0, |v| v + 1));
    println!("{:?}", half(9).map_or_else(|e| e.len() as i32, |v| v));
    println!("{:?} {:?}", half(4).ok(), half(5).err());
    println!("{:?}", half(4).and(Ok::<&str, String>("next")));
    println!("{:?}", half(5).or(Ok::<i32, ()>(0)));
    println!("{:?} {}", half(5).unwrap_or_default(), half(6).unwrap_or_else(|_| 99));
    println!("{}", half(2).is_ok_and(|v| v == 1));
    println!("{}", half(3).is_err_and(|e| e.ends_with("odd")));
    println!("{:?}", half(10).iter().next());
    let r: Result<Option<i32>, String> = Some(4).map(half).transpose();
    let r2: Result<Option<i32>, String> = None.map(half).transpose();
    let r3: Result<Option<i32>, String> = Some(3).map(half).transpose();
    println!("{:?} {:?} {:?}", r, r2, r3);
    let o: Option<Result<i32, String>> = Ok(Some(5)).transpose();
    println!("{:?}", o);
    let nested: Option<Option<i32>> = Some(Some(3));
    let nested_none: Option<Option<i32>> = Some(None);
    println!("{:?} {:?}", nested.flatten(), nested_none.flatten());
    let rr: Result<Result<i32, String>, String> = Ok(Err("inner".into()));
    println!("{:?}", rr.and_then(|x| x));
    println!("{:?}", non_neg(-1).ok_or("negative"));
    println!("{:?}", non_neg(5).ok_or_else(|| format!("neg")));
    println!("{:?}", non_neg(3).zip(non_neg(4)));
    println!("{:?}", non_neg(3).zip(non_neg(-4)));
    println!("{:?} {:?}", Some(1).xor(None::<i32>), Some(1).xor(Some(2)));
    println!("{:?} {:?}", Some(10).filter(|x| x % 2 == 0), Some(11).filter(|x| x % 2 == 0));
    let mut o1 = Some(5);
    let taken = o1.take();
    println!("{:?} {:?}", taken, o1);
    let mut o2 = Some(1);
    let old = o2.replace(2);
    println!("{:?} {:?}", old, o2);
    let mut cache: Option<Vec<i32>> = None;
    cache.get_or_insert_with(Vec::new).push(1);
    cache.get_or_insert_with(|| vec![100]).push(2);
    println!("{:?}", cache);
    let mut c2: Option<i32> = None;
    *c2.get_or_insert(5) += 1;
    println!("{:?}", c2);
    let mut c3 = Some(3);
    if let Some(x) = c3.as_mut() {
        *x *= 7;
    }
    println!("{:?} {:?}", c3, c3.as_ref().map(|x| x + 1));
    println!("{}", Some(4).is_some_and(|x| x > 3));
    println!("{}", None::<i32>.is_none_or(|x| x > 3));
    let mut seen = vec![];
    let v = Some(9).inspect(|x| seen.push(*x)).map(|x| x * 2);
    println!("{:?} {:?}", v, seen);
    let res: Result<i32, String> = Ok(3);
    let s = res.as_ref().map(|x| x.to_string()).unwrap_or_default();
    println!("{} {:?}", s, res);
    let words = ["3", "x", "7", "", "11"];
    let sum: i32 = words.iter().filter_map(|w| w.parse::<i32>().ok()).sum();
    let first_err = words.iter().map(|w| w.parse::<i32>()).find_map(Result::err);
    println!("{} {:?}", sum, first_err.map(|e| e.to_string()));
    let chained = Some("42")
        .and_then(|s| s.parse::<i32>().ok())
        .filter(|n| *n > 40)
        .map(|n| n * 2)
        .and_then(|n| if n < 100 { Some(n) } else { None })
        .unwrap_or(-1);
    println!("{}", chained);
    let opt_ref: Option<&String> = None;
    println!("{:?}", opt_ref.cloned().unwrap_or_else(|| "default".to_string()));
    let x: Option<&i32> = Some(&5);
    println!("{:?} {:?}", x.copied(), x.map(|v| *v as f64 / 2.0));
    let r4: Result<&str, &str> = Err("e");
    println!("{:?} {:?}", r4.unwrap_or("fallback"), r4.err().map(str::len));
    let ok: Result<u8, std::convert::Infallible> = Ok(3);
    let Ok(three) = ok;
    println!("{}", three);
    println!("{:?}", Some(2).map_or_else(|| -1, |v| v * v));
    println!("{:?}", Ok::<_, ()>(vec![1, 2]).map(|v| v.len()).iter().sum::<usize>());
    let deep: Result<Option<Result<i32, String>>, String> = Ok(Some(Ok(1)));
    println!("{:?}", deep.map(|o| o.transpose()).and_then(|r| r.map_err(|e| e)));
    println!("{:?}", Some(Some(Some(1))).flatten().flatten());
    println!("{:?}", Ok::<Result<i32, String>, String>(Ok(4)).flatten());
}
