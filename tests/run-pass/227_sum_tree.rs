// A simplified SumTree (as in Zed's sum_tree crate): a B-tree-like structure where each node
// caches a summary of its children, allowing seeking by any summary dimension.
use std::fmt::Debug;
use std::ops::{Add, AddAssign};
use std::sync::Arc;

trait Summary: Clone + Default + Debug {
    fn add_summary(&mut self, other: &Self);
}

trait Item: Clone + Debug {
    type Summary: Summary;
    fn summary(&self) -> Self::Summary;
}

trait Dimension<S: Summary>: Clone + Default + Debug + Ord {
    fn add_summary(&mut self, s: &S);
}

const BRANCH: usize = 3;

#[derive(Clone, Debug)]
enum Node<T: Item> {
    Leaf { items: Vec<T>, summary: T::Summary },
    Internal { children: Vec<Arc<Node<T>>>, summary: T::Summary, height: u8 },
}

impl<T: Item> Node<T> {
    fn summary(&self) -> &T::Summary {
        match self {
            Node::Leaf { summary, .. } | Node::Internal { summary, .. } => summary,
        }
    }
    fn height(&self) -> u8 {
        match self {
            Node::Leaf { .. } => 0,
            Node::Internal { height, .. } => *height,
        }
    }
}

struct SumTree<T: Item> {
    root: Arc<Node<T>>,
}

impl<T: Item> SumTree<T> {
    fn from_items(items: Vec<T>) -> Self {
        let mut level: Vec<Arc<Node<T>>> = items
            .chunks(BRANCH)
            .map(|c| {
                let mut s = T::Summary::default();
                for it in c {
                    s.add_summary(&it.summary());
                }
                Arc::new(Node::Leaf { items: c.to_vec(), summary: s })
            })
            .collect();
        if level.is_empty() {
            level.push(Arc::new(Node::Leaf { items: vec![], summary: Default::default() }));
        }
        let mut height = 0;
        while level.len() > 1 {
            height += 1;
            level = level
                .chunks(BRANCH)
                .map(|c| {
                    let mut s = T::Summary::default();
                    for n in c {
                        s.add_summary(n.summary());
                    }
                    Arc::new(Node::Internal { children: c.to_vec(), summary: s, height })
                })
                .collect();
        }
        SumTree { root: level.pop().unwrap() }
    }

    fn summary(&self) -> &T::Summary {
        self.root.summary()
    }

    /// Find the item at which dimension D first exceeds `target`; returns (item, D-start of item).
    fn seek<D: Dimension<T::Summary>>(&self, target: &D) -> Option<(T, D)> {
        let mut pos = D::default();
        let mut node = &self.root;
        loop {
            match &**node {
                Node::Internal { children, .. } => {
                    let mut next = None;
                    for c in children {
                        let mut end = pos.clone();
                        end.add_summary(c.summary());
                        if &end > target {
                            next = Some(c);
                            break;
                        }
                        pos = end;
                    }
                    node = next?;
                }
                Node::Leaf { items, .. } => {
                    for it in items {
                        let mut end = pos.clone();
                        end.add_summary(&it.summary());
                        if &end > target {
                            return Some((it.clone(), pos));
                        }
                        pos = end;
                    }
                    return None;
                }
            }
        }
    }

    fn items(&self) -> Vec<T> {
        fn walk<T: Item>(n: &Node<T>, out: &mut Vec<T>) {
            match n {
                Node::Leaf { items, .. } => out.extend(items.iter().cloned()),
                Node::Internal { children, .. } => children.iter().for_each(|c| walk(c, out)),
            }
        }
        let mut v = vec![];
        walk(&self.root, &mut v);
        v
    }
}

// Text chunks summarized by (chars, bytes, lines)
#[derive(Clone, Debug)]
struct Chunk(&'static str);

#[derive(Clone, Default, Debug, PartialEq)]
struct TextSummary {
    chars: usize,
    bytes: usize,
    lines: usize,
}

impl Summary for TextSummary {
    fn add_summary(&mut self, o: &Self) {
        self.chars += o.chars;
        self.bytes += o.bytes;
        self.lines += o.lines;
    }
}

impl Item for Chunk {
    type Summary = TextSummary;
    fn summary(&self) -> TextSummary {
        TextSummary { chars: self.0.chars().count(), bytes: self.0.len(), lines: self.0.matches('\n').count() }
    }
}

#[derive(Clone, Default, Debug, PartialEq, Eq, PartialOrd, Ord, Copy)]
struct CharOffset(usize);
#[derive(Clone, Default, Debug, PartialEq, Eq, PartialOrd, Ord, Copy)]
struct ByteOffset(usize);
#[derive(Clone, Default, Debug, PartialEq, Eq, PartialOrd, Ord, Copy)]
struct Line(usize);

impl Dimension<TextSummary> for CharOffset {
    fn add_summary(&mut self, s: &TextSummary) {
        self.0 += s.chars;
    }
}
impl Dimension<TextSummary> for ByteOffset {
    fn add_summary(&mut self, s: &TextSummary) {
        self.0 += s.bytes;
    }
}
impl Dimension<TextSummary> for Line {
    fn add_summary(&mut self, s: &TextSummary) {
        self.0 += s.lines;
    }
}

// Numbers summarized by (count, sum, max)
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Count(usize);
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Sum(i64);

impl Add for Sum {
    type Output = Sum;
    fn add(self, o: Sum) -> Sum {
        Sum(self.0 + o.0)
    }
}
impl AddAssign for Sum {
    fn add_assign(&mut self, o: Sum) {
        self.0 += o.0;
    }
}

#[derive(Clone, Default, Debug)]
struct NumSummary {
    count: Count,
    sum: Sum,
    max: i64,
}
impl Summary for NumSummary {
    fn add_summary(&mut self, o: &Self) {
        self.count.0 += o.count.0;
        self.sum += o.sum;
        self.max = self.max.max(o.max);
    }
}
impl Item for i64 {
    type Summary = NumSummary;
    fn summary(&self) -> NumSummary {
        NumSummary { count: Count(1), sum: Sum(*self), max: *self }
    }
}
impl Dimension<NumSummary> for Count {
    fn add_summary(&mut self, s: &NumSummary) {
        self.0 += s.count.0;
    }
}
impl Dimension<NumSummary> for Sum {
    fn add_summary(&mut self, s: &NumSummary) {
        *self = *self + s.sum;
    }
}

fn main() {
    let chunks = vec![
        Chunk("fn main"),
        Chunk("() {\n"),
        Chunk("    let"),
        Chunk(" x = \"é\";"),
        Chunk("\n    pr"),
        Chunk("intln!"),
        Chunk("(\"{x}\");\n"),
        Chunk("}\n"),
    ];
    let t = SumTree::from_items(chunks);
    println!("summary {:?} height {}", t.summary(), t.root.height());
    for c in [0, 7, 12, 20, 100] {
        println!("char {:>3} -> {:?}", c, t.seek(&CharOffset(c)));
    }
    for b in [17, 18, 19] {
        println!("byte {:>3} -> {:?}", b, t.seek(&ByteOffset(b)));
    }
    for l in [0, 1, 2, 3] {
        println!("line {} -> {:?}", l, t.seek(&Line(l)).map(|(c, _)| c.0));
    }

    let nums = SumTree::from_items((1..=20i64).map(|x| (x * 37) % 23 - 5).collect());
    println!("items {:?}", nums.items());
    println!("summary {:?} height {}", nums.summary(), nums.root.height());
    println!("index 7 -> {:?}", nums.seek(&Count(7)));
    println!("prefix sum > 40 at {:?}", nums.seek(&Sum(40)));
    println!("count 25 -> {:?}", nums.seek(&Count(25)));
    let empty: SumTree<i64> = SumTree::from_items(vec![]);
    println!("empty {:?} {:?}", empty.summary(), empty.seek(&Count(0)));
}
