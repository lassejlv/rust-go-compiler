trait Graph {
    fn nodes(&self) -> impl Iterator<Item = u32> + '_;
    fn neighbors(&self, n: u32) -> impl Iterator<Item = u32> + '_;
    fn degree(&self, n: u32) -> usize { self.neighbors(n).count() }
    fn edges(&self) -> Vec<(u32, u32)> {
        let mut v = vec![];
        for n in self.nodes() { for m in self.neighbors(n) { v.push((n, m)); } }
        v
    }
}

struct AdjList(Vec<Vec<u32>>);
impl Graph for AdjList {
    fn nodes(&self) -> impl Iterator<Item = u32> + '_ { 0..self.0.len() as u32 }
    fn neighbors(&self, n: u32) -> impl Iterator<Item = u32> + '_ { self.0[n as usize].iter().copied() }
}

struct Ring(u32);
impl Graph for Ring {
    fn nodes(&self) -> impl Iterator<Item = u32> + '_ { (0..self.0).rev() }
    fn neighbors(&self, n: u32) -> impl Iterator<Item = u32> + '_ {
        let k = self.0;
        [(n + 1) % k, (n + k - 1) % k].into_iter()
    }
}

trait Source {
    type Out;
    fn produce(&self, n: usize) -> impl Iterator<Item = Self::Out>;
    fn collect_n(&self, n: usize) -> Vec<Self::Out> { self.produce(n).collect() }
}
struct Squares;
impl Source for Squares {
    type Out = u64;
    fn produce(&self, n: usize) -> impl Iterator<Item = u64> { (1..=n as u64).map(|x| x * x) }
}
struct Letters(&'static str);
impl Source for Letters {
    type Out = char;
    fn produce(&self, n: usize) -> impl Iterator<Item = char> { self.0.chars().cycle().take(n) }
}

trait Factory { fn make(&self) -> impl Fn(i32) -> i32; }
struct Scale(i32);
impl Factory for Scale { fn make(&self) -> impl Fn(i32) -> i32 { let k = self.0; move |x| x * k } }

fn total_degree<G: Graph>(g: &G) -> usize { g.nodes().map(|n| g.degree(n)).sum() }

fn main() {
    let g = AdjList(vec![vec![1, 2], vec![2], vec![0], vec![]]);
    println!("{:?} {}", g.edges(), total_degree(&g));
    println!("{:?}", g.nodes().map(|n| g.degree(n)).collect::<Vec<_>>());
    let r = Ring(4);
    println!("{:?} {}", r.edges(), total_degree(&r));
    println!("{:?}", Squares.collect_n(5));
    println!("{:?}", Letters("ab").collect_n(5));
    println!("{}", Squares.produce(10).filter(|x| x % 2 == 1).sum::<u64>());
    let f = Scale(3).make();
    println!("{} {}", f(4), Scale(-1).make()(9));
}
