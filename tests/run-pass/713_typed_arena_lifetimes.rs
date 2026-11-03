// A typed arena allocator handing out &'a T references; a graph of nodes
// that reference each other through Cell<Option<&'a Node<'a>>>, plus a
// string interner that returns &'a str borrowed from the arena.
use std::cell::{Cell, RefCell};
use std::fmt;

struct Arena<T> {
    chunks: RefCell<Vec<Vec<T>>>,
    chunk_cap: usize,
}

impl<T> Arena<T> {
    fn new(chunk_cap: usize) -> Self {
        Arena { chunks: RefCell::new(vec![Vec::with_capacity(chunk_cap)]), chunk_cap }
    }
    fn alloc(&self, value: T) -> &T {
        let mut chunks = self.chunks.borrow_mut();
        if chunks.last().unwrap().len() == self.chunk_cap {
            let cap = self.chunk_cap;
            chunks.push(Vec::with_capacity(cap));
        }
        let last = chunks.last_mut().unwrap();
        last.push(value);
        let p: *const T = last.last().unwrap();
        // Safe: chunks never reallocate (capacity is fixed) and are never dropped
        // while the arena is alive.
        unsafe { &*p }
    }
    fn len(&self) -> usize { self.chunks.borrow().iter().map(Vec::len).sum() }
    fn chunk_count(&self) -> usize { self.chunks.borrow().len() }
}

struct Node<'a> {
    name: &'a str,
    weight: Cell<u32>,
    next: Cell<Option<&'a Node<'a>>>,
    kids: RefCell<Vec<&'a Node<'a>>>,
}

impl<'a> fmt::Debug for Node<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Node")
            .field("name", &self.name)
            .field("weight", &self.weight.get())
            .field("next", &self.next.get().map(|n| n.name))
            .field("kids", &self.kids.borrow().iter().map(|k| k.name).collect::<Vec<_>>())
            .finish()
    }
}

struct Interner<'a> {
    arena: &'a Arena<String>,
    seen: RefCell<Vec<&'a str>>,
}

impl<'a> Interner<'a> {
    fn intern(&self, s: &str) -> &'a str {
        if let Some(&x) = self.seen.borrow().iter().find(|&&x| x == s) {
            return x;
        }
        let stored: &'a str = self.arena.alloc(s.to_string()).as_str();
        self.seen.borrow_mut().push(stored);
        stored
    }
}

fn total_weight<'a>(n: &'a Node<'a>, depth: usize) -> u32 {
    if depth > 10 { return 0; }
    n.weight.get() + n.kids.borrow().iter().map(|k| total_weight(k, depth + 1)).sum::<u32>()
}

fn longest<'a, 'b>(a: &'a Node<'a>, b: &'b Node<'b>) -> &'a str where 'b: 'a {
    if a.name.len() >= b.name.len() { a.name } else { b.name }
}

fn walk_chain<'a>(start: &'a Node<'a>) -> impl Iterator<Item = &'a Node<'a>> + 'a {
    std::iter::successors(Some(start), |n| n.next.get()).take(8)
}

fn main() {
    let strings = Arena::new(4);
    let interner = Interner { arena: &strings, seen: RefCell::new(Vec::new()) };
    let nodes: Arena<Node> = Arena::new(3);
    let words = ["root", "left", "right", "leaf", "left", "root", "extra", "leaf"];
    let mut made: Vec<&Node> = Vec::new();
    for (i, w) in words.iter().enumerate() {
        let name = interner.intern(w);
        if made.iter().any(|n| std::ptr::eq(n.name, name)) {
            continue;
        }
        let n = nodes.alloc(Node { name, weight: Cell::new(i as u32 + 1), next: Cell::new(None), kids: RefCell::new(vec![]) });
        made.push(n);
    }
    println!("interned {} strings in {} chunks; {} nodes in {} chunks", strings.len(), strings.chunk_count(), nodes.len(), nodes.chunk_count());
    let find = |s: &str| *made.iter().find(|n| n.name == s).unwrap();
    let (root, left, right, leaf, extra) = (find("root"), find("left"), find("right"), find("leaf"), find("extra"));
    root.kids.borrow_mut().extend([left, right]);
    left.kids.borrow_mut().push(leaf);
    right.kids.borrow_mut().extend([leaf, extra]);
    for w in made.windows(2) {
        w[0].next.set(Some(w[1]));
    }
    // create a cycle in the chain
    extra.next.set(Some(left));
    for n in &made {
        println!("{:?}", n);
    }
    println!("total weight from root: {}", total_weight(root, 0));
    leaf.weight.set(100);
    println!("after bump: {}", total_weight(root, 0));
    let chain: Vec<&str> = walk_chain(root).map(|n| n.name).collect();
    println!("chain: {}", chain.join(" -> "));
    println!("longest: {}", longest(root, extra));
    let same = interner.intern("leaf");
    println!("interned same ptr: {}", std::ptr::eq(same, leaf.name));
    let many: Vec<&str> = (0..6).map(|i| interner.intern(&format!("gen{}", i % 4))).collect();
    println!("many: {:?} arena now {} strings / {} chunks", many, strings.len(), strings.chunk_count());
    let heavy: Vec<_> = made.iter().filter(|n| n.weight.get() > 2).map(|n| (n.name, n.weight.get())).collect();
    println!("heavy: {:?}", heavy);
}
