// Persistent (immutable, structurally shared) data structures built on Rc:
// an AVL map with path copying and version history, a rope for text with
// O(log n) concatenation/split and undo by keeping old versions, and a
// persistent stack/queue pair; sharing is verified with Rc::ptr_eq and
// strong counts, and results are cross-checked against std collections.
use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::fmt::Debug;
use std::rc::Rc;

// ---------------- persistent AVL map ----------------
#[derive(Debug)]
struct Node<K, V> {
    k: K,
    v: V,
    h: u8,
    size: usize,
    l: Link<K, V>,
    r: Link<K, V>,
}
type Link<K, V> = Option<Rc<Node<K, V>>>;

fn h<K, V>(n: &Link<K, V>) -> u8 {
    n.as_ref().map_or(0, |n| n.h)
}
fn sz<K, V>(n: &Link<K, V>) -> usize {
    n.as_ref().map_or(0, |n| n.size)
}

fn mk<K: Clone, V: Clone>(k: K, v: V, l: Link<K, V>, r: Link<K, V>) -> Rc<Node<K, V>> {
    Rc::new(Node { h: 1 + h(&l).max(h(&r)), size: 1 + sz(&l) + sz(&r), k, v, l, r })
}

fn rotate_right<K: Clone, V: Clone>(n: &Rc<Node<K, V>>) -> Rc<Node<K, V>> {
    let l = n.l.as_ref().unwrap();
    mk(l.k.clone(), l.v.clone(), l.l.clone(), Some(mk(n.k.clone(), n.v.clone(), l.r.clone(), n.r.clone())))
}
fn rotate_left<K: Clone, V: Clone>(n: &Rc<Node<K, V>>) -> Rc<Node<K, V>> {
    let r = n.r.as_ref().unwrap();
    mk(r.k.clone(), r.v.clone(), Some(mk(n.k.clone(), n.v.clone(), n.l.clone(), r.l.clone())), r.r.clone())
}

fn balance<K: Clone, V: Clone>(n: Rc<Node<K, V>>) -> Rc<Node<K, V>> {
    let bf = h(&n.l) as i32 - h(&n.r) as i32;
    if bf > 1 {
        let l = n.l.as_ref().unwrap();
        if h(&l.l) < h(&l.r) {
            let nl = rotate_left(l);
            return rotate_right(&mk(n.k.clone(), n.v.clone(), Some(nl), n.r.clone()));
        }
        return rotate_right(&n);
    }
    if bf < -1 {
        let r = n.r.as_ref().unwrap();
        if h(&r.r) < h(&r.l) {
            let nr = rotate_right(r);
            return rotate_left(&mk(n.k.clone(), n.v.clone(), n.l.clone(), Some(nr)));
        }
        return rotate_left(&n);
    }
    n
}

fn insert<K: Ord + Clone, V: Clone>(t: &Link<K, V>, k: K, v: V) -> Rc<Node<K, V>> {
    match t {
        None => mk(k, v, None, None),
        Some(n) => match k.cmp(&n.k) {
            Ordering::Less => balance(mk(n.k.clone(), n.v.clone(), Some(insert(&n.l, k, v)), n.r.clone())),
            Ordering::Greater => balance(mk(n.k.clone(), n.v.clone(), n.l.clone(), Some(insert(&n.r, k, v)))),
            Ordering::Equal => mk(k, v, n.l.clone(), n.r.clone()),
        },
    }
}

fn remove_min<K: Ord + Clone, V: Clone>(n: &Rc<Node<K, V>>) -> (Link<K, V>, (K, V)) {
    match &n.l {
        None => (n.r.clone(), (n.k.clone(), n.v.clone())),
        Some(l) => {
            let (nl, min) = remove_min(l);
            (Some(balance(mk(n.k.clone(), n.v.clone(), nl, n.r.clone()))), min)
        }
    }
}

fn remove<K: Ord + Clone, V: Clone>(t: &Link<K, V>, k: &K) -> Link<K, V> {
    let n = t.as_ref()?;
    Some(match k.cmp(&n.k) {
        Ordering::Less => balance(mk(n.k.clone(), n.v.clone(), remove(&n.l, k), n.r.clone())),
        Ordering::Greater => balance(mk(n.k.clone(), n.v.clone(), n.l.clone(), remove(&n.r, k))),
        Ordering::Equal => match (&n.l, &n.r) {
            (None, r) => return r.clone(),
            (l, None) => return l.clone(),
            (l, Some(r)) => {
                let (nr, (mk_, mv)) = remove_min(r);
                balance(mk(mk_, mv, l.clone(), nr))
            }
        },
    })
}

fn get<'a, K: Ord, V>(t: &'a Link<K, V>, k: &K) -> Option<&'a V> {
    let mut cur = t;
    while let Some(n) = cur {
        match k.cmp(&n.k) {
            Ordering::Less => cur = &n.l,
            Ordering::Greater => cur = &n.r,
            Ordering::Equal => return Some(&n.v),
        }
    }
    None
}

fn nth<K, V>(t: &Link<K, V>, mut i: usize) -> Option<(&K, &V)> {
    let mut cur = t;
    while let Some(n) = cur {
        let ls = sz(&n.l);
        match i.cmp(&ls) {
            Ordering::Less => cur = &n.l,
            Ordering::Equal => return Some((&n.k, &n.v)),
            Ordering::Greater => {
                i -= ls + 1;
                cur = &n.r;
            }
        }
    }
    None
}

fn to_vec<K: Clone, V: Clone>(t: &Link<K, V>, out: &mut Vec<(K, V)>) {
    if let Some(n) = t {
        to_vec(&n.l, out);
        out.push((n.k.clone(), n.v.clone()));
        to_vec(&n.r, out);
    }
}

fn count_nodes<K, V>(t: &Link<K, V>, seen: &mut Vec<*const Node<K, V>>) {
    if let Some(n) = t {
        let p = Rc::as_ptr(n);
        if !seen.contains(&p) {
            seen.push(p);
            count_nodes(&n.l, seen);
            count_nodes(&n.r, seen);
        }
    }
}

fn check_avl<K: Ord, V>(t: &Link<K, V>) -> bool {
    match t {
        None => true,
        Some(n) => {
            let ok_bal = (h(&n.l) as i32 - h(&n.r) as i32).abs() <= 1;
            let ok_ord = n.l.as_ref().map_or(true, |l| l.k < n.k) && n.r.as_ref().map_or(true, |r| r.k > n.k);
            ok_bal && ok_ord && n.size == 1 + sz(&n.l) + sz(&n.r) && check_avl(&n.l) && check_avl(&n.r)
        }
    }
}

// ---------------- rope ----------------
#[derive(Debug)]
enum Rope {
    Leaf(Rc<str>),
    Cat { len: usize, depth: u32, l: Rc<Rope>, r: Rc<Rope> },
}

impl Rope {
    fn len(&self) -> usize {
        match self {
            Rope::Leaf(s) => s.chars().count(),
            Rope::Cat { len, .. } => *len,
        }
    }
    fn depth(&self) -> u32 {
        match self {
            Rope::Leaf(_) => 0,
            Rope::Cat { depth, .. } => *depth,
        }
    }
    fn leaf(s: &str) -> Rc<Rope> {
        Rc::new(Rope::Leaf(Rc::from(s)))
    }
    fn concat(l: &Rc<Rope>, r: &Rc<Rope>) -> Rc<Rope> {
        if l.len() == 0 {
            return r.clone();
        }
        if r.len() == 0 {
            return l.clone();
        }
        if let (Rope::Leaf(a), Rope::Leaf(b)) = (&**l, &**r) {
            if a.len() + b.len() <= 16 {
                return Rope::leaf(&format!("{}{}", a, b));
            }
        }
        Rc::new(Rope::Cat { len: l.len() + r.len(), depth: 1 + l.depth().max(r.depth()), l: l.clone(), r: r.clone() })
    }
    fn split(t: &Rc<Rope>, at: usize) -> (Rc<Rope>, Rc<Rope>) {
        match &**t {
            Rope::Leaf(s) => {
                let b = s.char_indices().nth(at).map_or(s.len(), |(i, _)| i);
                (Rope::leaf(&s[..b]), Rope::leaf(&s[b..]))
            }
            Rope::Cat { l, r, .. } => {
                let ll = l.len();
                if at < ll {
                    let (a, b) = Rope::split(l, at);
                    (a, Rope::concat(&b, r))
                } else if at > ll {
                    let (a, b) = Rope::split(r, at - ll);
                    (Rope::concat(l, &a), b)
                } else {
                    (l.clone(), r.clone())
                }
            }
        }
    }
    fn insert(t: &Rc<Rope>, at: usize, s: &str) -> Rc<Rope> {
        let (a, b) = Rope::split(t, at);
        Rope::concat(&Rope::concat(&a, &Rope::leaf(s)), &b)
    }
    fn delete(t: &Rc<Rope>, from: usize, to: usize) -> Rc<Rope> {
        let (a, rest) = Rope::split(t, from);
        let (_, c) = Rope::split(&rest, to - from);
        Rope::concat(&a, &c)
    }
    fn collect(&self, out: &mut String) {
        match self {
            Rope::Leaf(s) => out.push_str(s),
            Rope::Cat { l, r, .. } => {
                l.collect(out);
                r.collect(out);
            }
        }
    }
    fn text(&self) -> String {
        let mut s = String::new();
        self.collect(&mut s);
        s
    }
    fn char_at(&self, i: usize) -> Option<char> {
        match self {
            Rope::Leaf(s) => s.chars().nth(i),
            Rope::Cat { l, r, .. } => {
                let ll = l.len();
                if i < ll {
                    l.char_at(i)
                } else {
                    r.char_at(i - ll)
                }
            }
        }
    }
}

// ---------------- persistent stack / banker's queue ----------------
#[derive(Clone, Debug)]
enum PStack<T> {
    Nil,
    Cons(T, Rc<PStack<T>>),
}

impl<T: Clone + Debug> PStack<T> {
    fn push(self: &Rc<Self>, x: T) -> Rc<Self> {
        Rc::new(PStack::Cons(x, self.clone()))
    }
    fn to_vec(&self) -> Vec<T> {
        let mut v = vec![];
        let mut cur = self;
        while let PStack::Cons(x, next) = cur {
            v.push(x.clone());
            cur = next;
        }
        v
    }
}

#[derive(Clone)]
struct PQueue<T> {
    front: Rc<PStack<T>>,
    back: Rc<PStack<T>>,
}

impl<T: Clone + Debug> PQueue<T> {
    fn new() -> Self {
        PQueue { front: Rc::new(PStack::Nil), back: Rc::new(PStack::Nil) }
    }
    fn push(&self, x: T) -> Self {
        PQueue { front: self.front.clone(), back: self.back.push(x) }
    }
    fn pop(&self) -> Option<(T, Self)> {
        let front = if let PStack::Nil = *self.front {
            let mut f = Rc::new(PStack::Nil);
            for x in self.back.to_vec() {
                f = f.push(x);
            }
            f
        } else {
            self.front.clone()
        };
        match &*front {
            PStack::Nil => None,
            PStack::Cons(x, rest) => Some((x.clone(), PQueue { front: rest.clone(), back: if let PStack::Nil = *self.front { Rc::new(PStack::Nil) } else { self.back.clone() } })),
        }
    }
}

fn main() {
    // AVL versions
    let mut versions: Vec<Link<u32, String>> = vec![None];
    let mut model: Vec<BTreeMap<u32, String>> = vec![BTreeMap::new()];
    let mut x = 12345u32;
    for step in 0..400 {
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        let k = x % 150;
        let prev = versions.last().unwrap().clone();
        let mut m = model.last().unwrap().clone();
        let next = if step % 4 == 3 {
            m.remove(&k);
            remove(&prev, &k)
        } else {
            m.insert(k, format!("v{}", step));
            Some(insert(&prev, k, format!("v{}", step)))
        };
        versions.push(next);
        model.push(m);
    }
    let mut all_ok = true;
    for (t, m) in versions.iter().zip(&model) {
        let mut v = vec![];
        to_vec(t, &mut v);
        let mv: Vec<(u32, String)> = m.iter().map(|(k, v)| (*k, v.clone())).collect();
        all_ok &= v == mv && check_avl(t) && sz(t) == m.len();
    }
    let last = versions.last().unwrap();
    println!("versions {} all match model & balanced: {} size {} height {}", versions.len(), all_ok, sz(last), h(last));
    println!("get 42 in v100 {:?} / last {:?}; nth(0) {:?} nth(10) {:?} nth(999) {:?}", get(&versions[100], &42), get(last, &42), nth(last, 0), nth(last, 10).map(|(k, _)| *k), nth(last, 999));
    let mut seen = vec![];
    for v in &versions {
        count_nodes(v, &mut seen);
    }
    let total_if_copied: usize = versions.iter().map(sz).sum();
    println!("distinct nodes across all versions {} vs {} if fully copied", seen.len(), total_if_copied);
    let a = Some(insert(&None, 1u8, 'a'));
    let b = Some(insert(&a, 2, 'b'));
    let c = Some(insert(&b, 0, 'z'));
    println!("rc counts a={} b={} c={} shared-left={}", Rc::strong_count(a.as_ref().unwrap()), Rc::strong_count(b.as_ref().unwrap()), Rc::strong_count(c.as_ref().unwrap()), Rc::ptr_eq(b.as_ref().unwrap().r.as_ref().unwrap(), c.as_ref().unwrap().r.as_ref().unwrap()));

    // rope editing with undo history
    let mut hist: Vec<Rc<Rope>> = vec![Rope::leaf("")];
    let mut plain: Vec<String> = vec![String::new()];
    let edits: Vec<(&str, usize, &str)> = vec![
        ("ins", 0, "Hello world"),
        ("ins", 5, ","),
        ("ins", 12, "! This is a rope-based text buffer"),
        ("del", 0, "7"),
        ("ins", 0, "Oh, h"),
        ("ins", 9999, " (with ünïcödé)"),
        ("del", 10, "20"),
        ("ins", 3, "+++"),
    ];
    for (op, at, arg) in edits {
        let cur = hist.last().unwrap().clone();
        let p = plain.last().unwrap().clone();
        let at = at.min(cur.len());
        let (next, np) = if op == "ins" {
            let mut s: Vec<char> = p.chars().collect();
            for (i, ch) in arg.chars().enumerate() {
                s.insert(at + i, ch);
            }
            (Rope::insert(&cur, at, arg), s.into_iter().collect::<String>())
        } else {
            let to = (at + arg.parse::<usize>().unwrap()).min(cur.len());
            let s: String = p.chars().enumerate().filter(|(i, _)| *i < at || *i >= to).map(|(_, c)| c).collect();
            (Rope::delete(&cur, at, to), s)
        };
        assert_eq!(next.text(), np);
        hist.push(next);
        plain.push(np);
    }
    let fin = hist.last().unwrap();
    println!("rope {:?} len {} depth {} char_at(7) {:?}", fin.text(), fin.len(), fin.depth(), fin.char_at(7));
    for v in [1, 3, 5] {
        println!("  undo to v{}: {:?}", v, hist[v].text());
    }
    // big rope from many small pieces
    let mut big = Rope::leaf("");
    for i in 0..500 {
        big = Rope::concat(&big, &Rope::leaf(&format!("[{}]", i)));
    }
    let (l, r) = Rope::split(&big, 1000);
    println!("big len {} depth {} split {} + {} char {:?}{:?}", big.len(), big.depth(), l.len(), r.len(), l.char_at(999), r.char_at(0));

    // persistent queue
    let q0: PQueue<u32> = PQueue::new();
    let q1 = q0.push(1).push(2).push(3);
    let (a1, q2) = q1.pop().unwrap();
    let q3 = q2.push(4);
    let (a2, q4) = q3.pop().unwrap();
    let (b1, _) = q1.pop().unwrap(); // old version still usable
    let mut drained = vec![];
    let mut q = q4.clone();
    while let Some((x, nq)) = q.pop() {
        drained.push(x);
        q = nq;
    }
    println!("queue pops {} {} old {} rest {:?} empty {}", a1, a2, b1, drained, q0.pop().is_none());
    let s0: Rc<PStack<&str>> = Rc::new(PStack::Nil);
    let s1 = s0.push("a").push("b");
    let s2 = s1.push("c");
    let s3 = s1.push("d");
    println!("{:?} {:?} {:?} shared tail rc={}", s2.to_vec(), s3.to_vec(), s1.to_vec(), Rc::strong_count(&s1));
}
