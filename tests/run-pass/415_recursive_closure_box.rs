// Recursive closures via Box<dyn Fn>, Rc<RefCell<Option<...>>> self-reference,
// a Y-combinator-style fixpoint with &dyn Fn, and recursion through struct-held closures.
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

fn fix<'a, A, R>(f: &'a dyn Fn(&dyn Fn(A) -> R, A) -> R) -> impl Fn(A) -> R + 'a {
    fn helper<A, R>(f: &dyn Fn(&dyn Fn(A) -> R, A) -> R, a: A) -> R {
        f(&|x| helper(f, x), a)
    }
    move |a| helper(f, a)
}

struct Rec<'a> {
    f: &'a dyn Fn(&Rec, u64) -> u64,
}
impl<'a> Rec<'a> {
    fn call(&self, n: u64) -> u64 {
        (self.f)(self, n)
    }
}

type Shared = Rc<RefCell<Option<Box<dyn Fn(u32) -> u32>>>>;

fn main() {
    // fixpoint factorial and fibonacci
    let fact_body = |rec: &dyn Fn(u64) -> u64, n: u64| if n <= 1 { 1 } else { n * rec(n - 1) };
    let fact = fix(&fact_body);
    println!("fact: {:?}", (0..8).map(|n| fact(n)).collect::<Vec<_>>());

    let fib_body = |rec: &dyn Fn(u32) -> u32, n: u32| if n < 2 { n } else { rec(n - 1) + rec(n - 2) };
    let fib = fix(&fib_body);
    println!("fib(15) = {}", fib(15));

    // struct-held self-referential call
    let gcd = Rec { f: &|r, n| if n == 0 { 0 } else { n % 10 + r.call(n / 10) } };
    println!("digit sum = {}", gcd.call(987654321));

    // Rc<RefCell<Option<Box<dyn Fn>>>> self reference
    let slot: Shared = Rc::new(RefCell::new(None));
    let weak_slot = Rc::downgrade(&slot);
    *slot.borrow_mut() = Some(Box::new(move |n| {
        if n == 0 {
            0
        } else {
            let s = weak_slot.upgrade().unwrap();
            let b = s.borrow();
            let f = b.as_ref().unwrap();
            n + f(n - 1)
        }
    }));
    let tri = slot.borrow().as_ref().unwrap()(10);
    println!("triangular(10) = {}", tri);

    // recursive tree walk with explicit Box<dyn Fn> passed down
    #[derive(Debug)]
    enum Tree {
        Leaf(i32),
        Node(Vec<Tree>),
    }
    let t = Tree::Node(vec![
        Tree::Leaf(1),
        Tree::Node(vec![Tree::Leaf(2), Tree::Leaf(3), Tree::Node(vec![])]),
        Tree::Node(vec![Tree::Node(vec![Tree::Leaf(4)])]),
    ]);
    fn walk(t: &Tree, depth: usize, visit: &dyn Fn(i32, usize) -> String, out: &mut Vec<String>) {
        match t {
            Tree::Leaf(v) => out.push(visit(*v, depth)),
            Tree::Node(ch) => {
                for c in ch {
                    walk(c, depth + 1, visit, out);
                }
            }
        }
    }
    let mut out = Vec::new();
    walk(&t, 0, &|v, d| format!("{}@{}", v, d), &mut out);
    println!("{}", out.join(" "));

    // memoized recursive closure via HashMap in RefCell
    let memo: RefCell<HashMap<u64, u64>> = RefCell::new(HashMap::new());
    let calls = std::cell::Cell::new(0);
    let collatz_body = |rec: &dyn Fn(u64) -> u64, n: u64| -> u64 {
        calls.set(calls.get() + 1);
        if n == 1 {
            return 0;
        }
        if let Some(&v) = memo.borrow().get(&n) {
            return v;
        }
        let next = if n % 2 == 0 { n / 2 } else { 3 * n + 1 };
        let v = 1 + rec(next);
        memo.borrow_mut().insert(n, v);
        v
    };
    let collatz = fix(&collatz_body);
    let best = (1..30u64).max_by_key(|&n| (collatz(n), std::cmp::Reverse(n))).unwrap();
    println!("longest collatz under 30: {} steps={}", best, collatz(best));
    println!("memo size {} calls {}", memo.borrow().len(), calls.get());

    // mutual recursion through two boxed closures held in a struct
    struct EvenOdd {
        even: Box<dyn Fn(&EvenOdd, u32) -> bool>,
        odd: Box<dyn Fn(&EvenOdd, u32) -> bool>,
    }
    let eo = EvenOdd {
        even: Box::new(|s, n| if n == 0 { true } else { (s.odd)(s, n - 1) }),
        odd: Box::new(|s, n| if n == 0 { false } else { (s.even)(s, n - 1) }),
    };
    let flags: Vec<String> = (0..6).map(|n| format!("{}:{}", n, if (eo.even)(&eo, n) { "E" } else { "O" })).collect();
    println!("{}", flags.join(" "));
}
