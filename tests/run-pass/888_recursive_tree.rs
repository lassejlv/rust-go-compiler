use std::fmt::Debug;

#[derive(Debug)]
enum Tree<T: Ord> {
    Leaf,
    Node(Box<Node<T>>),
}

#[derive(Debug)]
struct Node<T: Ord> {
    val: T,
    left: Tree<T>,
    right: Tree<T>,
}

impl<T: Ord + Debug + Clone> Tree<T> {
    fn new() -> Self { Tree::Leaf }
    fn insert(&mut self, v: T) -> bool {
        match self {
            Tree::Leaf => {
                *self = Tree::Node(Box::new(Node { val: v, left: Tree::Leaf, right: Tree::Leaf }));
                true
            }
            Tree::Node(n) => {
                if v < n.val { n.left.insert(v) } else if v > n.val { n.right.insert(v) } else { false }
            }
        }
    }
    fn contains(&self, v: &T) -> bool {
        match self {
            Tree::Leaf => false,
            Tree::Node(n) => {
                if *v == n.val { true } else if *v < n.val { n.left.contains(v) } else { n.right.contains(v) }
            }
        }
    }
    fn inorder(&self, out: &mut Vec<T>) {
        if let Tree::Node(n) = self {
            n.left.inorder(out);
            out.push(n.val.clone());
            n.right.inorder(out);
        }
    }
    fn height(&self) -> usize {
        match self {
            Tree::Leaf => 0,
            Tree::Node(n) => 1 + n.left.height().max(n.right.height()),
        }
    }
    fn min(&self) -> Option<&T> {
        match self {
            Tree::Leaf => None,
            Tree::Node(n) => n.left.min().or(Some(&n.val)),
        }
    }
    fn fold<A>(&self, init: A, f: &impl Fn(A, &T) -> A) -> A {
        match self {
            Tree::Leaf => init,
            Tree::Node(n) => {
                let a = n.left.fold(init, f);
                let a = f(a, &n.val);
                n.right.fold(a, f)
            }
        }
    }
    fn map<U: Ord, F: Fn(&T) -> U + Copy>(&self, f: F) -> Tree<U> {
        match self {
            Tree::Leaf => Tree::Leaf,
            Tree::Node(n) => Tree::Node(Box::new(Node { val: f(&n.val), left: n.left.map(f), right: n.right.map(f) })),
        }
    }
}

// Rose tree
#[derive(Debug, Clone)]
struct Rose<T> { label: T, kids: Vec<Rose<T>> }
impl<T: Clone + Debug> Rose<T> {
    fn leaf(t: T) -> Self { Rose { label: t, kids: vec![] } }
    fn size(&self) -> usize { 1 + self.kids.iter().map(|k| k.size()).sum::<usize>() }
    fn flatten(&self) -> Vec<T> {
        let mut v = vec![self.label.clone()];
        for k in &self.kids { v.extend(k.flatten()); }
        v
    }
    fn render(&self, depth: usize, out: &mut String) {
        out.push_str(&format!("{}{:?}\n", "  ".repeat(depth), self.label));
        for k in &self.kids { k.render(depth + 1, out); }
    }
}

// Linked list via Option<Box<>>
#[derive(Debug)]
struct List<T> { head: Option<Box<ListNode<T>>> }
#[derive(Debug)]
struct ListNode<T> { v: T, next: Option<Box<ListNode<T>>> }
impl<T: Debug> List<T> {
    fn push(&mut self, v: T) { let old = self.head.take(); self.head = Some(Box::new(ListNode { v, next: old })); }
    fn pop(&mut self) -> Option<T> { self.head.take().map(|n| { self.head = n.next; n.v }) }
    fn reverse(&mut self) {
        let mut prev: Option<Box<ListNode<T>>> = None;
        let mut cur = self.head.take();
        while let Some(mut n) = cur { cur = n.next.take(); n.next = prev; prev = Some(n); }
        self.head = prev;
    }
    fn to_vec(&self) -> Vec<&T> {
        let mut v = vec![]; let mut c = &self.head;
        while let Some(n) = c { v.push(&n.v); c = &n.next; }
        v
    }
}

fn main() {
    let mut t = Tree::new();
    for x in [50, 30, 70, 20, 40, 60, 80, 30, 65] {
        let ins = t.insert(x);
        if !ins { println!("dup {}", x); }
    }
    let mut v = vec![];
    t.inorder(&mut v);
    println!("{:?}", v);
    println!("h={} min={:?} has40={} has45={}", t.height(), t.min(), t.contains(&40), t.contains(&45));
    println!("sum={}", t.fold(0, &|a, x| a + x));
    let ts = t.map(|x| format!("n{}", x % 7));
    let mut sv = vec![];
    ts.inorder(&mut sv);
    println!("{:?}", sv);
    let mut st: Tree<String> = Tree::new();
    for w in "the quick brown fox jumps over the lazy dog".split(' ') { st.insert(w.to_string()); }
    println!("{:?} {}", st.min(), st.fold(String::new(), &|a, s| a + &s[..1]));
    let r = Rose { label: "root", kids: vec![Rose { label: "a", kids: vec![Rose::leaf("a1"), Rose::leaf("a2")] }, Rose::leaf("b")] };
    println!("{} {:?}", r.size(), r.flatten());
    let mut s = String::new();
    r.render(0, &mut s);
    print!("{}", s);
    let mut l = List { head: None };
    for i in 1..=4 { l.push(i * i); }
    println!("{:?}", l.to_vec());
    l.reverse();
    println!("{:?}", l.to_vec());
    let p = l.pop();
    println!("pop={:?}", p);
    println!("{:?}", l.to_vec());
}
