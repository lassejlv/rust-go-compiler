use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread;

#[derive(Debug, Default)]
struct Account {
    balance: i64,
    ops: u32,
}

fn main() {
    let counter = Arc::new(Mutex::new(0u64));
    let mut handles = vec![];
    for i in 0..10 {
        let c = Arc::clone(&counter);
        handles.push(thread::spawn(move || {
            for _ in 0..1000 {
                *c.lock().unwrap() += 1;
            }
            i
        }));
    }
    let ids: u32 = handles.into_iter().map(|h| h.join().unwrap()).sum();
    println!("final count = {}", *counter.lock().unwrap());
    println!("sum of ids = {}", ids);
    println!("strong count = {}", Arc::strong_count(&counter));

    let account = Arc::new(Mutex::new(Account::default()));
    let handles: Vec<_> = (0..5)
        .map(|i| {
            let acc = Arc::clone(&account);
            thread::spawn(move || {
                for k in 0..100 {
                    let mut a = acc.lock().unwrap();
                    if (k + i) % 2 == 0 {
                        a.balance += 10;
                    } else {
                        a.balance -= 3;
                    }
                    a.ops += 1;
                }
            })
        })
        .collect();
    for h in handles {
        h.join().unwrap();
    }
    let a = account.lock().unwrap();
    println!("{:?}", *a);
    drop(a);

    let words = Arc::new(Mutex::new(HashMap::<String, usize>::new()));
    let texts = vec!["a b c a", "b c d", "a a d e", "e e e"];
    let hs: Vec<_> = texts
        .into_iter()
        .map(|t| {
            let w = Arc::clone(&words);
            let t = t.to_string();
            thread::spawn(move || {
                for word in t.split(' ') {
                    *w.lock().unwrap().entry(word.to_string()).or_insert(0) += 1;
                }
            })
        })
        .collect();
    for h in hs {
        h.join().unwrap();
    }
    let map = words.lock().unwrap();
    let mut entries: Vec<_> = map.iter().collect();
    entries.sort();
    println!("{:?}", entries);
    drop(map);

    let shared = Arc::new(Mutex::new(Vec::new()));
    let hs: Vec<_> = (0..6)
        .map(|i| {
            let s = Arc::clone(&shared);
            thread::spawn(move || s.lock().unwrap().push(i * i))
        })
        .collect();
    hs.into_iter().for_each(|h| h.join().unwrap());
    let mut v = Arc::try_unwrap(shared).unwrap().into_inner().unwrap();
    v.sort();
    println!("{:?}", v);

    let m = Mutex::new(5);
    {
        let mut g = m.lock().unwrap();
        *g *= 3;
    }
    println!("try_lock ok: {}", m.try_lock().is_ok());
    let g = m.lock().unwrap();
    println!("try_lock while held fails: {}", m.try_lock().is_err());
    println!("value {}", *g);
    drop(g);
    println!("into_inner {}", m.into_inner().unwrap());
}
