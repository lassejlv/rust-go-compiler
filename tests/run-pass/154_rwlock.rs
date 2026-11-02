use std::collections::BTreeMap;
use std::sync::{Arc, RwLock};
use std::thread;

struct Config {
    values: BTreeMap<String, i32>,
    version: u32,
}

fn main() {
    let cfg = Arc::new(RwLock::new(Config { values: BTreeMap::new(), version: 0 }));
    {
        let mut w = cfg.write().unwrap();
        w.values.insert("threads".into(), 4);
        w.values.insert("retries".into(), 3);
        w.version += 1;
    }

    let readers: Vec<_> = (0..4)
        .map(|i| {
            let c = Arc::clone(&cfg);
            thread::spawn(move || {
                let r = c.read().unwrap();
                let sum: i32 = r.values.values().sum();
                sum * (i + 1)
            })
        })
        .collect();
    let results: Vec<i32> = readers.into_iter().map(|h| h.join().unwrap()).collect();
    println!("reader results {:?}", results);

    let writers: Vec<_> = (0..5)
        .map(|i| {
            let c = Arc::clone(&cfg);
            thread::spawn(move || {
                let mut w = c.write().unwrap();
                *w.values.entry(format!("key{}", i % 2)).or_insert(0) += i;
                w.version += 1;
            })
        })
        .collect();
    for h in writers {
        h.join().unwrap();
    }
    let r = cfg.read().unwrap();
    println!("version {}", r.version);
    for (k, v) in r.values.iter() {
        println!("{} = {}", k, v);
    }
    // multiple simultaneous read guards are allowed
    let r2 = cfg.read().unwrap();
    println!("two readers see same version: {}", r.version == r2.version);
    println!("try_write while reading fails: {}", cfg.try_write().is_err());
    drop(r);
    drop(r2);
    println!("try_write after release ok: {}", cfg.try_write().is_ok());

    let lock = RwLock::new(vec![1, 2, 3]);
    {
        let g = lock.read().unwrap();
        println!("len {}", g.len());
    }
    lock.write().unwrap().push(4);
    println!("{:?}", *lock.read().unwrap());
    let inner = lock.into_inner().unwrap();
    println!("inner {:?}", inner);
}
