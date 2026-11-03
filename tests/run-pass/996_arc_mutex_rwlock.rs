use std::sync::{Arc, Mutex, RwLock};
use std::thread;
use std::collections::BTreeMap;

fn main() {
    let counter = Arc::new(Mutex::new(0u64));
    let mut hs = vec![];
    for i in 0..10 {
        let c = Arc::clone(&counter);
        hs.push(thread::spawn(move || {
            for _ in 0..100 {
                *c.lock().unwrap() += i;
            }
        }));
    }
    for h in hs { h.join().unwrap(); }
    println!("counter {}", *counter.lock().unwrap());
    println!("strong {}", Arc::strong_count(&counter));

    let map = Arc::new(RwLock::new(BTreeMap::new()));
    let writers: Vec<_> = (0..4).map(|i| {
        let m = Arc::clone(&map);
        thread::spawn(move || {
            let mut w = m.write().unwrap();
            w.insert(i, format!("v{}", i * i));
        })
    }).collect();
    for w in writers { w.join().unwrap(); }
    let readers: Vec<_> = (0..4).map(|i| {
        let m = Arc::clone(&map);
        thread::spawn(move || {
            let r = m.read().unwrap();
            r.get(&i).cloned()
        })
    }).collect();
    let mut got: Vec<_> = readers.into_iter().map(|h| h.join().unwrap()).collect();
    got.sort();
    println!("{:?}", got);
    println!("{:?}", map.read().unwrap());

    let m = Mutex::new(vec![1]);
    {
        let mut g = m.lock().unwrap();
        g.push(2);
        println!("try_lock while held: {}", m.try_lock().is_err());
    }
    println!("try_lock after: {}", m.try_lock().is_ok());
    let inner = m.into_inner().unwrap();
    println!("{:?}", inner);
    let rw = RwLock::new(5);
    {
        let r1 = rw.read().unwrap();
        let r2 = rw.read().unwrap();
        println!("two readers {} {}", *r1, *r2);
        println!("try_write blocked: {}", rw.try_write().is_err());
    }
    *rw.write().unwrap() += 1;
    println!("rw {}", rw.into_inner().unwrap());
    let poisoned = Arc::new(Mutex::new(1));
    let p2 = poisoned.clone();
    let r = thread::spawn(move || {
        let _g = p2.lock().unwrap();
        panic!("poison it");
    }).join();
    println!("join err {}", r.is_err());
    println!("poisoned {}", poisoned.is_poisoned());
    let v = match poisoned.lock() {
        Ok(g) => *g,
        Err(e) => *e.into_inner() + 100,
    };
    println!("recovered {}", v);
    let mut mm = Mutex::new(3);
    *mm.get_mut().unwrap() = 4;
    println!("get_mut {}", mm.lock().unwrap());
}
