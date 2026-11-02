// Cell (get/set/replace/take/swap/update-like), Cell in structs shared by &, OnceCell lazy init,
// OnceLock in a static, LazyCell/LazyLock, thread_local with Cell, and Cell<Option<T>> patterns.
use std::cell::{Cell, LazyCell, OnceCell};
use std::sync::{LazyLock, OnceLock};

static CONFIG: OnceLock<Vec<&'static str>> = OnceLock::new();
static SQUARES: LazyLock<Vec<u64>> = LazyLock::new(|| {
    println!("  (init squares)");
    (0..10).map(|i| i * i).collect()
});

thread_local! {
    static DEPTH: Cell<u32> = const { Cell::new(0) };
}

fn config() -> &'static Vec<&'static str> {
    CONFIG.get_or_init(|| {
        println!("  (init config)");
        vec!["debug", "fast"]
    })
}

struct Counter<'a> {
    hits: &'a Cell<u32>,
    name: &'static str,
}
impl Counter<'_> {
    fn hit(&self) -> u32 {
        self.hits.set(self.hits.get() + 1);
        self.hits.get()
    }
}

struct Node {
    id: u32,
    visited: Cell<bool>,
    cost: Cell<Option<u32>>,
}

fn recurse(n: u32) -> u32 {
    DEPTH.with(|d| d.set(d.get() + 1));
    let r = if n == 0 { DEPTH.with(|d| d.get()) } else { recurse(n - 1) };
    DEPTH.with(|d| d.set(d.get() - 1));
    r
}

fn main() {
    let c = Cell::new(5);
    let old = c.replace(10);
    c.set(c.get() * 2);
    println!("{} {}", old, c.get());
    let a = Cell::new(1);
    let b = Cell::new(2);
    a.swap(&b);
    println!("{} {} take={} after={}", a.get(), b.get(), a.take(), a.get());
    let s = Cell::new(String::from("cell string"));
    let inner = s.take();
    s.set(inner.to_uppercase());
    println!("{}", s.into_inner());

    let shared = Cell::new(0);
    let c1 = Counter { hits: &shared, name: "one" };
    let c2 = Counter { hits: &shared, name: "two" };
    c1.hit();
    c2.hit();
    c1.hit();
    println!("{} {} total={}", c1.name, c2.name, shared.get());

    let nodes: Vec<Node> = (0..4).map(|id| Node { id, visited: Cell::new(false), cost: Cell::new(None) }).collect();
    for n in nodes.iter().filter(|n| n.id % 2 == 0) {
        n.visited.set(true);
        n.cost.set(Some(n.id * 10));
    }
    for n in &nodes {
        let cost = n.cost.take();
        println!("node {} visited={} cost={:?} now={:?}", n.id, n.visited.get(), cost, n.cost.get());
    }

    let cells: Vec<Cell<i32>> = vec![Cell::new(1), Cell::new(2), Cell::new(3)];
    let slice: &[Cell<i32>] = &cells;
    for w in slice.windows(2) {
        w[1].set(w[1].get() + w[0].get());
    }
    println!("{:?}", cells.iter().map(Cell::get).collect::<Vec<_>>());
    let mut arr = [1, 2, 3];
    let cell_slice = Cell::from_mut(&mut arr[..]).as_slice_of_cells();
    cell_slice[0].set(100);
    cell_slice[2].swap(&cell_slice[1]);
    println!("{:?}", arr);

    let once: OnceCell<String> = OnceCell::new();
    println!("{:?}", once.get());
    let v1 = once.get_or_init(|| "first".to_string());
    println!("{}", v1);
    let v2 = once.get_or_init(|| "second".to_string());
    println!("{} set_again={:?}", v2, once.set("third".into()));
    let mut once2: OnceCell<u8> = OnceCell::new();
    println!("{:?} {:?} {:?}", once2.set(1), once2.take(), once2.get());

    println!("before config");
    println!("{:?}", config());
    println!("{:?}", config().len());
    println!("before squares");
    println!("{} {}", SQUARES[3], SQUARES.iter().sum::<u64>());

    let lazy = LazyCell::new(|| {
        println!("  (lazy cell eval)");
        42
    });
    println!("lazy created");
    println!("{} {}", *lazy, *lazy + 1);

    println!("depth {}", recurse(5));
    println!("depth after {}", DEPTH.with(|d| d.get()));
}
