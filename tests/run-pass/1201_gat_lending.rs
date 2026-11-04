// Generic associated types and projections in where-clauses
trait LendingIter {
    type Item<'a>
    where
        Self: 'a;
    fn next<'a>(&'a mut self) -> Option<Self::Item<'a>>;
}

struct WindowsMut<'s> {
    data: &'s mut [i32],
    pos: usize,
    size: usize,
}

impl<'s> LendingIter for WindowsMut<'s> {
    type Item<'a> = &'a mut [i32] where Self: 'a;
    fn next<'a>(&'a mut self) -> Option<&'a mut [i32]> {
        if self.pos + self.size > self.data.len() {
            return None;
        }
        let w = &mut self.data[self.pos..self.pos + self.size];
        self.pos += 1;
        Some(w)
    }
}

trait Container {
    type Elem;
    type Iter<'a>: Iterator<Item = &'a Self::Elem>
    where
        Self: 'a;
    fn elems<'a>(&'a self) -> Self::Iter<'a>;
}

struct Stack<T> {
    v: Vec<T>,
}

impl<T> Container for Stack<T> {
    type Elem = T;
    type Iter<'a> = std::iter::Rev<std::slice::Iter<'a, T>> where T: 'a;
    fn elems<'a>(&'a self) -> Self::Iter<'a> {
        self.v.iter().rev()
    }
}

fn sum_all<C>(c: &C) -> i64
where
    C: Container,
    C::Elem: Copy + Into<i64>,
{
    c.elems().map(|&e| e.into()).sum()
}

fn first_debug<C>(c: &C) -> String
where
    C: Container,
    for<'a> &'a C::Elem: std::fmt::Debug,
{
    match c.elems().next() {
        Some(e) => format!("{:?}", e),
        None => "empty".to_string(),
    }
}

trait Family {
    type Member<T>;
    fn wrap<T>(t: T) -> Self::Member<T>;
}

struct OptionFamily;
impl Family for OptionFamily {
    type Member<T> = Option<T>;
    fn wrap<T>(t: T) -> Option<T> {
        Some(t)
    }
}

struct VecFamily;
impl Family for VecFamily {
    type Member<T> = Vec<T>;
    fn wrap<T>(t: T) -> Vec<T> {
        vec![t]
    }
}

fn wrap_both<F: Family>(a: i32, b: &'static str) -> (F::Member<i32>, F::Member<&'static str>) {
    (F::wrap(a), F::wrap(b))
}

trait Graph {
    type Node: Copy + std::fmt::Debug + PartialEq;
    type Edges<'g>: Iterator<Item = Self::Node>
    where
        Self: 'g;
    fn neighbors<'g>(&'g self, n: Self::Node) -> Self::Edges<'g>;
}

struct Adj(Vec<Vec<usize>>);
impl Graph for Adj {
    type Node = usize;
    type Edges<'g> = std::iter::Copied<std::slice::Iter<'g, usize>>;
    fn neighbors<'g>(&'g self, n: usize) -> Self::Edges<'g> {
        self.0[n].iter().copied()
    }
}

fn reachable<G: Graph>(g: &G, start: G::Node) -> Vec<G::Node> {
    let mut seen = vec![start];
    let mut stack = vec![start];
    while let Some(n) = stack.pop() {
        for m in g.neighbors(n) {
            if !seen.contains(&m) {
                seen.push(m);
                stack.push(m);
            }
        }
    }
    seen
}

fn main() {
    let mut data = [1, 2, 3, 4, 5];
    let mut w = WindowsMut { data: &mut data, pos: 0, size: 3 };
    while let Some(win) = w.next() {
        win[0] += win[2];
        println!("{:?}", win);
    }
    println!("{:?}", data);

    let s = Stack { v: vec![1u8, 2, 3, 250] };
    println!("{}", sum_all(&s));
    println!("{}", first_debug(&s));
    let e: Stack<u16> = Stack { v: vec![] };
    println!("{}", first_debug(&e));

    let (a, b) = wrap_both::<OptionFamily>(5, "x");
    println!("{:?} {:?}", a, b);
    let (a, b) = wrap_both::<VecFamily>(6, "y");
    println!("{:?} {:?}", a, b);

    let g = Adj(vec![vec![1, 2], vec![3], vec![3, 0], vec![], vec![0]]);
    println!("{:?}", reachable(&g, 0));
    println!("{:?}", reachable(&g, 4));
}
