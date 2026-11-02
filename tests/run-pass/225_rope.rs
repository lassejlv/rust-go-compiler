// A persistent rope with Rc-shared nodes: concat, split, insert, delete, char/line queries.
use std::fmt;
use std::rc::Rc;

const MAX_LEAF: usize = 8;

enum Node {
    Leaf(String),
    Branch { left: Rc<Node>, right: Rc<Node>, len: usize, newlines: usize },
}

#[derive(Clone)]
struct Rope {
    root: Rc<Node>,
}

impl Node {
    fn len(&self) -> usize {
        match self {
            Node::Leaf(s) => s.chars().count(),
            Node::Branch { len, .. } => *len,
        }
    }
    fn newlines(&self) -> usize {
        match self {
            Node::Leaf(s) => s.matches('\n').count(),
            Node::Branch { newlines, .. } => *newlines,
        }
    }
    fn depth(&self) -> usize {
        match self {
            Node::Leaf(_) => 0,
            Node::Branch { left, right, .. } => 1 + left.depth().max(right.depth()),
        }
    }
    fn write_to(&self, out: &mut String) {
        match self {
            Node::Leaf(s) => out.push_str(s),
            Node::Branch { left, right, .. } => {
                left.write_to(out);
                right.write_to(out);
            }
        }
    }
    fn leaves(&self) -> usize {
        match self {
            Node::Leaf(_) => 1,
            Node::Branch { left, right, .. } => left.leaves() + right.leaves(),
        }
    }
}

fn branch(left: Rc<Node>, right: Rc<Node>) -> Rc<Node> {
    if left.len() == 0 {
        return right;
    }
    if right.len() == 0 {
        return left;
    }
    let len = left.len() + right.len();
    let newlines = left.newlines() + right.newlines();
    Rc::new(Node::Branch { left, right, len, newlines })
}

fn build(chars: &[char]) -> Rc<Node> {
    if chars.len() <= MAX_LEAF {
        Rc::new(Node::Leaf(chars.iter().collect()))
    } else {
        let mid = chars.len() / 2;
        branch(build(&chars[..mid]), build(&chars[mid..]))
    }
}

fn split(node: &Rc<Node>, at: usize) -> (Rc<Node>, Rc<Node>) {
    match &**node {
        Node::Leaf(s) => {
            let chars: Vec<char> = s.chars().collect();
            let at = at.min(chars.len());
            (build(&chars[..at]), build(&chars[at..]))
        }
        Node::Branch { left, right, .. } => {
            let ll = left.len();
            if at == ll {
                (left.clone(), right.clone())
            } else if at < ll {
                let (a, b) = split(left, at);
                (a, branch(b, right.clone()))
            } else {
                let (a, b) = split(right, at - ll);
                (branch(left.clone(), a), b)
            }
        }
    }
}

impl Rope {
    fn from_str(s: &str) -> Rope {
        let chars: Vec<char> = s.chars().collect();
        Rope { root: build(&chars) }
    }
    fn len(&self) -> usize {
        self.root.len()
    }
    fn concat(&self, o: &Rope) -> Rope {
        Rope { root: branch(self.root.clone(), o.root.clone()) }
    }
    fn insert(&self, at: usize, s: &str) -> Rope {
        let (a, b) = split(&self.root, at);
        let mid = Rope::from_str(s).root;
        Rope { root: branch(branch(a, mid), b) }
    }
    fn delete(&self, from: usize, to: usize) -> Rope {
        let (a, rest) = split(&self.root, from);
        let (_, c) = split(&rest, to - from);
        Rope { root: branch(a, c) }
    }
    fn char_at(&self, mut i: usize) -> Option<char> {
        let mut n = &self.root;
        loop {
            match &**n {
                Node::Leaf(s) => return s.chars().nth(i),
                Node::Branch { left, right, .. } => {
                    if i < left.len() {
                        n = left;
                    } else {
                        i -= left.len();
                        n = right;
                    }
                }
            }
        }
    }
    fn line_count(&self) -> usize {
        self.root.newlines() + 1
    }
    fn rebalance(&self) -> Rope {
        Rope::from_str(&self.to_string())
    }
}

impl fmt::Display for Rope {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut s = String::new();
        self.root.write_to(&mut s);
        f.write_str(&s)
    }
}

fn stats(label: &str, r: &Rope) {
    println!(
        "{:<8} len={:>3} lines={} depth={} leaves={} rc={}",
        label,
        r.len(),
        r.line_count(),
        r.root.depth(),
        r.root.leaves(),
        Rc::strong_count(&r.root)
    );
}

fn main() {
    let r = Rope::from_str("The quick brown fox\njumps over\nthe lazy dog.");
    stats("base", &r);
    println!("{:?}", r.to_string());
    let r2 = r.insert(10, "red ");
    stats("insert", &r2);
    println!("{:?}", r2.to_string());
    let r3 = r2.delete(4, 10);
    stats("delete", &r3);
    println!("{:?}", r3.to_string());
    println!("chars: {:?} {:?} {:?}", r3.char_at(0), r3.char_at(4), r3.char_at(1000));
    let mut big = Rope::from_str("");
    for i in 0..20 {
        big = big.concat(&Rope::from_str(&format!("[{}]", i)));
    }
    stats("big", &big);
    let bal = big.rebalance();
    stats("balanced", &bal);
    println!("{}", bal);
    let shared = big.clone();
    stats("shared", &shared);
    let edited = shared.insert(5, "\n").insert(0, "\n");
    stats("edited", &edited);
    println!("orig unchanged: {}", big.to_string() == bal.to_string());
    let uni = Rope::from_str("héllo wörld ünïcode ✓");
    let u2 = uni.delete(6, 12).insert(6, "→ ");
    println!("{} ({} chars)", u2, u2.len());
}
