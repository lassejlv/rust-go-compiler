// Cell, RefCell (borrow/try_borrow/borrow_mut, Ref::map), OnceCell, LazyCell, OnceLock, LazyLock, thread_local
use std::cell::{Cell, LazyCell, OnceCell, Ref, RefCell, RefMut};
use std::collections::HashMap;
use std::sync::{LazyLock, OnceLock};

struct Memo {
    calls: Cell<u32>,
    cache: RefCell<HashMap<u64, u64>>,
}
impl Memo {
    fn fib(&self, n: u64) -> u64 {
        self.calls.set(self.calls.get() + 1);
        if n < 2 {
            return n;
        }
        if let Some(&v) = self.cache.borrow().get(&n) {
            return v;
        }
        let v = self.fib(n - 1) + self.fib(n - 2);
        self.cache.borrow_mut().insert(n, v);
        v
    }
}

#[derive(Debug)]
struct Person {
    name: String,
    tags: Vec<String>,
}

static CONFIG: OnceLock<HashMap<&'static str, i32>> = OnceLock::new();
static SQUARES: LazyLock<Vec<u64>> = LazyLock::new(|| {
    println!("init SQUARES");
    (0..10).map(|x| x * x).collect()
});

thread_local! {
    static DEPTH: Cell<u32> = const { Cell::new(0) };
    static LOG: RefCell<Vec<String>> = RefCell::new(Vec::new());
}

fn recurse(n: u32) {
    DEPTH.with(|d| d.set(d.get() + 1));
    LOG.with_borrow_mut(|l| l.push(format!("enter {}", n)));
    if n > 0 {
        recurse(n - 1);
    }
}

fn main() {
    let c = Cell::new(5);
    c.set(c.get() * 2);
    let old = c.replace(1);
    println!("{} {} {}", old, c.get(), c.take());
    let cv = Cell::new(vec![1, 2]);
    let mut v = cv.take();
    v.push(3);
    cv.set(v);
    println!("{:?}", cv.into_inner());
    let c2 = Cell::new(3);
    let c3 = Cell::new(4);
    c2.swap(&c3);
    println!("{} {}", c2.get(), c3.get());
    let m = Memo { calls: Cell::new(0), cache: RefCell::new(HashMap::new()) };
    println!("fib(50)={} calls={}", m.fib(50), m.calls.get());

    let p = RefCell::new(Person { name: "ann".into(), tags: vec!["a".into()] });
    {
        let r1 = p.borrow();
        let r2 = p.borrow();
        println!("{} {} try_mut_err={}", r1.name, r2.tags.len(), p.try_borrow_mut().is_err());
    }
    {
        let mut w = p.borrow_mut();
        w.tags.push("b".into());
        println!("try_borrow_err={}", p.try_borrow().is_err());
    }
    let name: Ref<String> = Ref::map(p.borrow(), |p| &p.name);
    println!("mapped {}", *name);
    drop(name);
    {
        let mut tags: RefMut<Vec<String>> = RefMut::map(p.borrow_mut(), |p| &mut p.tags);
        tags.retain(|t| t != "a");
        tags.push("z".into());
    }
    let (n, t) = Ref::map_split(p.borrow(), |p| (&p.name, &p.tags));
    println!("{} {:?}", *n, *t);
    drop((n, t));
    let old = p.replace(Person { name: "bob".into(), tags: vec![] });
    println!("{:?} -> {:?}", old, p.borrow());
    p.replace_with(|old| Person { name: old.name.to_uppercase(), tags: vec!["x".into()] });
    println!("{:?}", p.into_inner());

    let once: OnceCell<String> = OnceCell::new();
    println!("{:?}", once.get());
    println!("{}", once.get_or_init(|| "first".to_string()));
    println!("{}", once.get_or_init(|| "second".to_string()));
    println!("{:?}", once.set("third".into()));
    let lazy = LazyCell::new(|| {
        println!("computing lazy");
        42
    });
    println!("before deref");
    println!("{} {}", *lazy, *lazy + 1);
    CONFIG.get_or_init(|| HashMap::from([("threads", 4), ("retries", 2)]));
    let mut keys: Vec<_> = CONFIG.get().unwrap().iter().collect();
    keys.sort();
    println!("{:?}", keys);
    println!("{:?}", CONFIG.set(HashMap::new()).is_err());
    println!("before squares");
    println!("{} {}", SQUARES[3], SQUARES.iter().sum::<u64>());
    recurse(3);
    println!("depth {}", DEPTH.with(Cell::get));
    LOG.with(|l| println!("{:?}", l.borrow()));
    let h = std::thread::spawn(|| {
        recurse(1);
        DEPTH.with(|d| d.get())
    });
    println!("thread depth {} main depth {}", h.join().unwrap(), DEPTH.get());
    let shared = RefCell::new(0);
    let incr = || *shared.borrow_mut() += 1;
    incr();
    incr();
    println!("{}", shared.borrow());
    let cells: Vec<Cell<i32>> = (0..4).map(Cell::new).collect();
    for w in cells.windows(2) {
        w[1].set(w[1].get() + w[0].get());
    }
    println!("{:?}", cells.iter().map(Cell::get).collect::<Vec<_>>());
    let slice_cells = Cell::from_mut(&mut [1, 2, 3][..]).as_slice_of_cells().len();
    println!("{}", slice_cells);
}
