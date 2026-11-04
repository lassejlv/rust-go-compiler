use std::collections::VecDeque;
use std::fmt::Debug;

fn constrain<F>(f: F) -> F where F: for<'a> Fn(&'a str) -> &'a str { f }
fn constrain2<F, T: ?Sized, U: ?Sized>(f: F) -> F where F: for<'a> Fn(&'a T) -> &'a U { f }

struct IterMut<'a, T> { rest: &'a mut [T] }
impl<'a, T> Iterator for IterMut<'a, T> {
    type Item = &'a mut T;
    fn next(&mut self) -> Option<&'a mut T> {
        let s = std::mem::take(&mut self.rest);
        let (first, rest) = s.split_first_mut()?;
        self.rest = rest;
        Some(first)
    }
}
impl<'a, T> DoubleEndedIterator for IterMut<'a, T> {
    fn next_back(&mut self) -> Option<&'a mut T> {
        let s = std::mem::take(&mut self.rest);
        let (last, rest) = s.split_last_mut()?;
        self.rest = rest;
        Some(last)
    }
}

struct Tree<T> { val: T, kids: Vec<Tree<T>> }
impl<T> Tree<T> {
    fn leaf(v: T) -> Self { Tree { val: v, kids: vec![] } }
    fn iter(&self) -> impl Iterator<Item = &T> + '_ {
        let mut stack = vec![self];
        std::iter::from_fn(move || {
            let n = stack.pop()?;
            stack.extend(n.kids.iter().rev());
            Some(&n.val)
        })
    }
    fn bfs_mut(&mut self, mut f: impl FnMut(usize, &mut T)) {
        let mut q: VecDeque<(usize, &mut Tree<T>)> = VecDeque::new();
        q.push_back((0, self));
        while let Some((d, n)) = q.pop_front() {
            f(d, &mut n.val);
            for k in n.kids.iter_mut() { q.push_back((d + 1, k)); }
        }
    }
    fn map<U>(self, f: &impl Fn(T) -> U) -> Tree<U> {
        Tree { val: f(self.val), kids: self.kids.into_iter().map(|k| k.map(f)).collect() }
    }
    fn fold<A>(&self, init: A, f: &mut dyn FnMut(A, &T) -> A) -> A {
        let mut acc = f(init, &self.val);
        for k in &self.kids { acc = k.fold(acc, f); }
        acc
    }
}

fn make_finalizer(items: Vec<String>) -> impl FnOnce() -> Vec<String> {
    move || { let mut v = items; v.sort(); v.dedup(); v }
}

fn run_once(fs: Vec<Box<dyn FnOnce(&mut Vec<i32>) + '_>>) -> Vec<i32> {
    let mut v = vec![];
    for f in fs { f(&mut v); }
    v
}

fn split_by<'a, T, P>(xs: &'a [T], pred: P) -> impl Iterator<Item = &'a [T]> + 'a where P: Fn(&T) -> bool + 'a {
    xs.split(move |x| pred(x)).filter(|s| !s.is_empty())
}

fn longest<'a, I>(it: I) -> Option<&'a str> where I: IntoIterator<Item = &'a str> {
    it.into_iter().fold(None, |best: Option<&'a str>, s| match best { Some(b) if b.len() >= s.len() => Some(b), _ => Some(s) })
}

fn main() {
    let tail = constrain(|s| &s[1..]);
    let owned = String::from("hello");
    println!("{} {}", tail(&owned), tail("xy"));
    let first = constrain2(|v: &Vec<i32>| &v[..1]);
    println!("{:?}", first(&vec![9, 8]));
    let fld = constrain2::<_, (String, u8), String>(|p| &p.0);
    println!("{}", fld(&("pair".to_string(), 1)));

    let mut arr = [1, 2, 3, 4, 5];
    for x in (IterMut { rest: &mut arr }) { *x *= 10; }
    let mut it = IterMut { rest: &mut arr };
    if let Some(l) = it.next_back() { *l += 1; }
    let collected: Vec<&mut i32> = it.rev().collect();
    println!("{:?}", collected);
    println!("{:?}", arr);

    let mut t = Tree { val: 1, kids: vec![Tree { val: 2, kids: vec![Tree::leaf(4), Tree::leaf(5)] }, Tree::leaf(3)] };
    println!("{:?}", t.iter().collect::<Vec<_>>());
    t.bfs_mut(|d, v| *v += d as i32 * 100);
    println!("{:?}", t.iter().copied().collect::<Vec<_>>());
    let ts = t.map(&|v| format!("n{}", v));
    println!("{:?}", ts.iter().collect::<Vec<_>>());
    let total_len = ts.fold(0, &mut |a, s: &String| a + s.len());
    let mut seen = vec![];
    ts.fold((), &mut |(), s| seen.push(s.clone()));
    println!("{} {:?}", total_len, seen);

    let fin = make_finalizer(vec!["b".into(), "a".into(), "b".into()]);
    println!("{:?}", fin());
    let extra = vec![7, 8];
    println!("{:?}", run_once(vec![Box::new(|v| v.push(1)), Box::new(move |v| v.extend(extra)), Box::new(|v| v.reverse())]));
    let data = [1, 0, 2, 3, 0, 0, 4];
    println!("{:?}", split_by(&data, |x| *x == 0).collect::<Vec<_>>());
    let words = vec!["aa".to_string(), "bbbb".to_string(), "cc".to_string()];
    println!("{:?}", longest(words.iter().map(|s| s.as_str())));
}
