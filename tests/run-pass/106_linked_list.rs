#[derive(Debug)]
enum List {
    Cons(i32, Box<List>),
    Nil,
}

use List::{Cons, Nil};

impl List {
    fn sum(&self) -> i32 {
        match self {
            Cons(v, rest) => v + rest.sum(),
            Nil => 0,
        }
    }
    fn to_string(&self) -> String {
        match self {
            Cons(v, rest) => match **rest {
                Nil => format!("{}", v),
                _ => format!("{} -> {}", v, rest.to_string()),
            },
            Nil => String::from("empty"),
        }
    }
}

struct Node {
    val: i32,
    next: Option<Box<Node>>,
}

struct Stack {
    head: Option<Box<Node>>,
    len: usize,
}

impl Stack {
    fn new() -> Self {
        Stack { head: None, len: 0 }
    }
    fn push(&mut self, val: i32) {
        let old = self.head.take();
        self.head = Some(Box::new(Node { val, next: old }));
        self.len += 1;
    }
    fn pop(&mut self) -> Option<i32> {
        self.head.take().map(|n| {
            self.head = n.next;
            self.len -= 1;
            n.val
        })
    }
    fn peek(&self) -> Option<&i32> {
        self.head.as_ref().map(|n| &n.val)
    }
    fn peek_mut(&mut self) -> Option<&mut i32> {
        self.head.as_mut().map(|n| &mut n.val)
    }
    fn reverse(&mut self) {
        let mut prev: Option<Box<Node>> = None;
        let mut cur = self.head.take();
        while let Some(mut node) = cur {
            cur = node.next.take();
            node.next = prev;
            prev = Some(node);
        }
        self.head = prev;
    }
    fn to_vec(&self) -> Vec<i32> {
        let mut out = Vec::new();
        let mut cur = &self.head;
        while let Some(node) = cur {
            out.push(node.val);
            cur = &node.next;
        }
        out
    }
}

fn main() {
    let list = Cons(1, Box::new(Cons(2, Box::new(Cons(3, Box::new(Nil))))));
    println!("{:?}", list);
    println!("sum = {}", list.sum());
    println!("str = {}", list.to_string());
    println!("nil = {}", Nil.to_string());
    let mut s = Stack::new();
    for i in 1..=5 {
        s.push(i * i);
    }
    println!("stack = {:?} len {}", s.to_vec(), s.len);
    println!("peek = {:?}", s.peek());
    if let Some(top) = s.peek_mut() {
        *top = -1;
    }
    println!("pop = {:?}", s.pop());
    s.reverse();
    println!("reversed = {:?}", s.to_vec());
    while let Some(v) = s.pop() {
        print!("{} ", v);
    }
    println!("| len {}", s.len);
    println!("pop empty = {:?}", s.pop());
    let mut big = Stack::new();
    for i in 0..10000 {
        big.push(i);
    }
    let mut total: i64 = 0;
    while let Some(v) = big.pop() {
        total += v as i64;
    }
    println!("big total = {}", total);
}
