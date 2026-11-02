use std::fmt::Display;

#[derive(Debug)]
struct Stack<T> {
    items: Vec<T>,
}

impl<T> Stack<T> {
    fn new() -> Self {
        Stack { items: Vec::new() }
    }
    fn push(&mut self, item: T) {
        self.items.push(item);
    }
    fn pop(&mut self) -> Option<T> {
        self.items.pop()
    }
    fn peek(&self) -> Option<&T> {
        self.items.last()
    }
    fn len(&self) -> usize {
        self.items.len()
    }
    fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

impl<T: Display> Stack<T> {
    fn render(&self) -> String {
        let parts: Vec<String> = self.items.iter().map(|i| i.to_string()).collect();
        format!("[{}>", parts.join("|"))
    }
}

#[derive(Debug, Clone, Copy)]
struct Pair<T> {
    a: T,
    b: T,
}

impl<T: PartialOrd + Copy> Pair<T> {
    fn larger(&self) -> T {
        if self.a > self.b { self.a } else { self.b }
    }
    fn swap(self) -> Pair<T> {
        Pair { a: self.b, b: self.a }
    }
}

struct KeyValue<K, V> {
    key: K,
    value: V,
}

impl<K: Display, V: Display> KeyValue<K, V> {
    fn show(&self) -> String {
        format!("{} => {}", self.key, self.value)
    }
}

impl<K, V> KeyValue<K, V> {
    fn map_value<W, F: Fn(V) -> W>(self, f: F) -> KeyValue<K, W> {
        KeyValue { key: self.key, value: f(self.value) }
    }
}

fn check_balanced(s: &str) -> bool {
    let mut st: Stack<char> = Stack::new();
    for c in s.chars() {
        match c {
            '(' | '[' | '{' => st.push(c),
            ')' | ']' | '}' => {
                let want = match c {
                    ')' => '(',
                    ']' => '[',
                    _ => '{',
                };
                if st.pop() != Some(want) {
                    return false;
                }
            }
            _ => {}
        }
    }
    st.is_empty()
}

fn main() {
    let mut s: Stack<i32> = Stack::new();
    for i in 1..=4 {
        s.push(i * 10);
    }
    println!("{} len={} peek={:?}", s.render(), s.len(), s.peek());
    println!("pop={:?} pop={:?}", s.pop(), s.pop());
    println!("{:?}", s);
    let mut names = Stack::new();
    names.push("x".to_string());
    names.push("y".to_string());
    println!("{}", names.render());
    let p = Pair { a: 3, b: 9 };
    println!("larger={} swapped={:?}", p.larger(), p.swap());
    let pf = Pair { a: 2.5, b: -1.0 };
    println!("larger f64 = {}", pf.larger());
    let kv = KeyValue { key: "answer", value: 42 };
    println!("{}", kv.show());
    let kv2 = kv.map_value(|v| format!("<{}>", v * 2));
    println!("{}", kv2.show());
    for t in ["(a[b]{c})", "(]", "((", "{[()()]}"] {
        println!("{} balanced: {}", t, check_balanced(t));
    }
}
