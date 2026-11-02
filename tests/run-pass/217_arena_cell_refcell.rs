// Typed arena + graph with Cell/RefCell interior mutability (like typed-arena / rustc's arenas).
use std::cell::{Cell, RefCell};
use std::fmt;

struct Arena<T> {
    chunks: RefCell<Vec<Vec<T>>>,
    chunk_cap: usize,
    allocated: Cell<usize>,
}

impl<T> Arena<T> {
    fn new(chunk_cap: usize) -> Self {
        Arena { chunks: RefCell::new(vec![Vec::with_capacity(chunk_cap)]), chunk_cap, allocated: Cell::new(0) }
    }

    #[allow(clippy::mut_from_ref)]
    fn alloc(&self, value: T) -> &mut T {
        let mut chunks = self.chunks.borrow_mut();
        let needs_new = {
            let last = chunks.last().unwrap();
            last.len() == last.capacity()
        };
        if needs_new {
            let cap = self.chunk_cap;
            chunks.push(Vec::with_capacity(cap));
        }
        let last = chunks.last_mut().unwrap();
        last.push(value);
        self.allocated.set(self.allocated.get() + 1);
        let p: *mut T = last.last_mut().unwrap();
        // Safe because chunks never reallocate (we never exceed capacity) and the arena outlives refs.
        unsafe { &mut *p }
    }

    fn len(&self) -> usize {
        self.allocated.get()
    }

    fn chunk_count(&self) -> usize {
        self.chunks.borrow().len()
    }
}

struct Node<'a> {
    name: String,
    edges: RefCell<Vec<&'a Node<'a>>>,
    visits: Cell<u32>,
}

impl<'a> Node<'a> {
    fn connect(&'a self, other: &'a Node<'a>) {
        self.edges.borrow_mut().push(other);
    }
}

impl fmt::Debug for Node<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let names: Vec<&str> = self.edges.borrow().iter().map(|n| n.name.as_str()).collect();
        write!(f, "{} -> {:?} (visits {})", self.name, names, self.visits.get())
    }
}

fn dfs<'a>(n: &'a Node<'a>, depth: usize, out: &mut Vec<String>) {
    n.visits.set(n.visits.get() + 1);
    if n.visits.get() > 1 {
        out.push(format!("{}{} (seen)", "  ".repeat(depth), n.name));
        return;
    }
    out.push(format!("{}{}", "  ".repeat(depth), n.name));
    for e in n.edges.borrow().iter() {
        dfs(e, depth + 1, out);
    }
}

// Cell-based counters & caches
struct Memo {
    cache: RefCell<Vec<Option<u64>>>,
    hits: Cell<u32>,
    misses: Cell<u32>,
}

impl Memo {
    fn new(n: usize) -> Self {
        Memo { cache: RefCell::new(vec![None; n]), hits: Cell::new(0), misses: Cell::new(0) }
    }
    fn ways(&self, n: usize) -> u64 {
        if n <= 1 {
            return 1;
        }
        if let Some(v) = self.cache.borrow()[n] {
            self.hits.set(self.hits.get() + 1);
            return v;
        }
        self.misses.set(self.misses.get() + 1);
        let v = self.ways(n - 1) + self.ways(n - 2) + if n >= 3 { self.ways(n - 3) } else { 0 };
        self.cache.borrow_mut()[n] = Some(v);
        v
    }
}

fn main() {
    let arena: Arena<Node> = Arena::new(3);
    let names = ["root", "a", "b", "c", "d", "e", "f"];
    let nodes: Vec<&Node> = names
        .iter()
        .map(|n| &*arena.alloc(Node { name: n.to_string(), edges: RefCell::new(vec![]), visits: Cell::new(0) }))
        .collect();
    nodes[0].connect(nodes[1]);
    nodes[0].connect(nodes[2]);
    nodes[1].connect(nodes[3]);
    nodes[2].connect(nodes[3]);
    nodes[3].connect(nodes[4]);
    nodes[4].connect(nodes[0]);
    nodes[2].connect(nodes[5]);
    nodes[5].connect(nodes[6]);
    println!("arena len={} chunks={}", arena.len(), arena.chunk_count());

    let mut out = vec![];
    dfs(nodes[0], 0, &mut out);
    for l in &out {
        println!("{}", l);
    }
    for n in &nodes {
        println!("{:?}", n);
    }

    let ints: Arena<i64> = Arena::new(2);
    let x = ints.alloc(5);
    let y = ints.alloc(7);
    *x += 100;
    *y *= 3;
    let z = ints.alloc(*x + *y);
    println!("{} {} {} chunks={}", x, y, z, ints.chunk_count());

    let m = Memo::new(40);
    println!("ways(30) = {}", m.ways(30));
    println!("hits={} misses={}", m.hits.get(), m.misses.get());
    println!("ways(35) = {}", m.ways(35));
    println!("hits={} misses={}", m.hits.get(), m.misses.get());

    let shared = RefCell::new(vec![1, 2, 3]);
    {
        let r1 = shared.borrow();
        let r2 = shared.borrow();
        println!("two borrows: {} {} try_mut_ok={}", r1.len(), r2[0], shared.try_borrow_mut().is_ok());
    }
    shared.borrow_mut().push(4);
    println!("{:?} try_mut_ok={}", shared.borrow(), shared.try_borrow_mut().is_ok());
    let c = Cell::new(10);
    let old = c.replace(20);
    c.set(c.get() + old);
    println!("cell {} take {} after {}", c.get(), c.take(), c.get());
}
