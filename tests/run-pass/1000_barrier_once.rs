use std::sync::{Arc, Barrier, Mutex, Once, OnceLock, LazyLock};
use std::thread;
use std::collections::HashMap;

static INIT: Once = Once::new();
static CONFIG: OnceLock<String> = OnceLock::new();
static TABLE: LazyLock<Vec<u32>> = LazyLock::new(|| {
    println!("computing TABLE");
    (0..10).map(|i| i * i).collect()
});
static NAMES: LazyLock<HashMap<&'static str, i32>> = LazyLock::new(|| {
    let mut m = HashMap::new();
    m.insert("one", 1);
    m.insert("two", 2);
    m
});

fn config() -> &'static str {
    CONFIG.get_or_init(|| {
        println!("init config");
        "cfg-v1".to_string()
    })
}

fn main() {
    let n = 5;
    let barrier = Arc::new(Barrier::new(n));
    let log = Arc::new(Mutex::new(Vec::new()));
    let hs: Vec<_> = (0..n).map(|i| {
        let b = barrier.clone();
        let l = log.clone();
        thread::spawn(move || {
            l.lock().unwrap().push(format!("before"));
            let res = b.wait();
            l.lock().unwrap().push(format!("after"));
            (i, res.is_leader())
        })
    }).collect();
    let results: Vec<_> = hs.into_iter().map(|h| h.join().unwrap()).collect();
    let leaders = results.iter().filter(|r| r.1).count();
    println!("leaders {}", leaders);
    let l = log.lock().unwrap();
    let first_after = l.iter().position(|s| s == "after").unwrap();
    println!("all befores first: {}", first_after == n);

    for _ in 0..3 {
        INIT.call_once(|| println!("Once ran"));
    }
    println!("once completed {}", INIT.is_completed());
    println!("config get before: {:?}", CONFIG.get());
    let hs: Vec<_> = (0..4).map(|_| thread::spawn(|| config().len())).collect();
    for h in hs { assert_eq!(h.join().unwrap(), 6); }
    println!("{}", config());
    println!("set again: {:?}", CONFIG.set("x".into()));
    println!("before TABLE");
    println!("{}", TABLE[7]);
    println!("{:?}", TABLE.iter().rev().take(2).collect::<Vec<_>>());
    println!("two={}", NAMES["two"]);
    let cell: OnceLock<Vec<i32>> = OnceLock::new();
    println!("{:?}", cell.get_or_init(|| vec![1, 2]));
    let mut cell2 = cell;
    cell2.get_mut().unwrap().push(3);
    println!("{:?}", cell2.into_inner());
    let lazy = LazyLock::new(|| 40 + 2);
    println!("lazy {}", *lazy);
    let oc = std::cell::OnceCell::new();
    println!("{}", oc.get_or_init(|| "oncecell"));
    println!("{:?}", oc.get());
}
