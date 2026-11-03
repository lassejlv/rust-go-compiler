use std::fmt::Debug;

#[derive(Debug, Clone)]
struct Stack<T> {
    items: Vec<T>,
}

impl<T: Clone + Debug> Stack<T> {
    fn new() -> Self { Stack { items: Vec::new() } }
    fn push(&mut self, t: T) -> &mut Self { self.items.push(t); self }
    fn map<U, F: Fn(&T) -> U>(&self, f: F) -> Stack<U> {
        Stack { items: self.items.iter().map(f).collect() }
    }
    fn fold<A>(&self, init: A, f: impl Fn(A, &T) -> A) -> A {
        let mut acc = init;
        for x in &self.items { acc = f(acc, x); }
        acc
    }
    fn zip<U: Clone>(&self, other: &Stack<U>) -> Stack<(T, U)> {
        Stack { items: self.items.iter().cloned().zip(other.items.iter().cloned()).collect() }
    }
    fn find_by<K: PartialEq, F>(&self, key: K, f: F) -> Option<&T>
    where
        F: Fn(&T) -> K,
    {
        self.items.iter().find(|x| f(x) == key)
    }
    fn convert<U: From<T>>(self) -> Vec<U> {
        self.items.into_iter().map(U::from).collect()
    }
}

struct Wrapper<T>(T);
impl<T: Debug> Wrapper<T> {
    fn with<R>(&self, f: impl FnOnce(&T) -> R) -> R { f(&self.0) }
    fn pair_with<U: Debug>(&self, u: U) -> String { format!("({:?}, {:?})", self.0, u) }
}

fn main() {
    let mut s: Stack<i32> = Stack::new();
    s.push(1).push(2).push(3);
    let doubled = s.map(|x| x * 2);
    let strs = s.map(|x| format!("#{}", x));
    println!("{:?} {:?}", doubled, strs);
    println!("{}", s.fold(0, |a, x| a + x));
    println!("{}", strs.fold(String::new(), |a, x| a + x));
    let z = s.zip(&strs);
    println!("{:?}", z.items);
    println!("{:?}", z.find_by(2, |p| p.0));
    println!("{:?}", z.find_by("#3".to_string(), |p| p.1.clone()));
    println!("{:?}", z.find_by(9, |p| p.0));
    let small: Stack<u8> = Stack { items: vec![200, 100] };
    let big: Vec<u32> = small.clone().convert();
    let fl: Vec<f64> = small.convert();
    println!("{:?} {:?}", big, fl);
    let w = Wrapper(vec![1, 2]);
    println!("{}", w.with(|v| v.len()));
    println!("{}", w.pair_with("x"));
    println!("{}", Wrapper('c').pair_with(Some(1.5)));
}
