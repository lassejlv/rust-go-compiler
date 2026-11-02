trait Container {
    type Item;
    fn get(&self, i: usize) -> Option<&Self::Item>;
    fn first(&self) -> Option<&Self::Item> {
        self.get(0)
    }
    fn put(&mut self, item: Self::Item);
    fn size(&self) -> usize;
}

struct IntBox {
    items: Vec<i32>,
}

struct Words {
    words: Vec<String>,
}

impl Container for IntBox {
    type Item = i32;
    fn get(&self, i: usize) -> Option<&i32> {
        self.items.get(i)
    }
    fn put(&mut self, item: i32) {
        self.items.push(item);
    }
    fn size(&self) -> usize {
        self.items.len()
    }
}

impl Container for Words {
    type Item = String;
    fn get(&self, i: usize) -> Option<&String> {
        self.words.get(i)
    }
    fn put(&mut self, item: String) {
        self.words.push(item.to_uppercase());
    }
    fn size(&self) -> usize {
        self.words.len()
    }
}

fn fill<C: Container>(c: &mut C, items: Vec<C::Item>) {
    for it in items {
        c.put(it);
    }
}

fn last_of<C>(c: &C) -> Option<&C::Item>
where
    C: Container,
{
    if c.size() == 0 { None } else { c.get(c.size() - 1) }
}

trait Graph {
    type Node: Copy + std::fmt::Debug;
    fn neighbors(&self, n: Self::Node) -> Vec<Self::Node>;
}

struct Ring {
    n: u32,
}

impl Graph for Ring {
    type Node = u32;
    fn neighbors(&self, x: u32) -> Vec<u32> {
        vec![(x + self.n - 1) % self.n, (x + 1) % self.n]
    }
}

fn walk<G: Graph>(g: &G, start: G::Node, steps: usize) -> Vec<G::Node> {
    let mut path = vec![start];
    let mut cur = start;
    for _ in 0..steps {
        cur = g.neighbors(cur)[1];
        path.push(cur);
    }
    path
}

fn main() {
    let mut ib = IntBox { items: vec![] };
    fill(&mut ib, vec![3, 1, 4]);
    println!("ib first={:?} last={:?} size={}", ib.first(), last_of(&ib), ib.size());
    let mut w = Words { words: vec![] };
    println!("empty last = {:?}", last_of(&w));
    fill(&mut w, vec!["alpha".to_string(), "omega".to_string()]);
    println!("w first={:?} get(1)={:?} get(5)={:?}", w.first(), w.get(1), w.get(5));
    let ring = Ring { n: 5 };
    println!("neighbors of 0: {:?}", ring.neighbors(0));
    println!("walk: {:?}", walk(&ring, 3, 6));
}
