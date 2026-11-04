mod y01 {
    struct Pipeline<'a> { stages: Vec<Box<dyn Fn(Box<dyn Iterator<Item = i64> + 'a>) -> Box<dyn Iterator<Item = i64> + 'a> + 'a>> }
    impl<'a> Pipeline<'a> {
        fn new() -> Self { Pipeline { stages: vec![] } }
        fn add(mut self, f: impl Fn(Box<dyn Iterator<Item = i64> + 'a>) -> Box<dyn Iterator<Item = i64> + 'a> + 'a) -> Self { self.stages.push(Box::new(f)); self }
        fn run(&self, input: Vec<i64>) -> Vec<i64> {
            let mut it: Box<dyn Iterator<Item = i64> + 'a> = Box::new(input.into_iter());
            for s in &self.stages { it = s(it); }
            it.collect()
        }
    }
    pub fn main() {
        let k = 3;
        let p = Pipeline::new()
            .add(|it| Box::new(it.filter(|x| x % 2 == 0)))
            .add(move |it| Box::new(it.map(move |x| x * k)))
            .add(|it| Box::new(it.rev_safe()))
            .add(|it| Box::new(it.take(4)));
        println!("{:?}", p.run((1..=20).collect()));
        let mut dynit: Box<dyn Iterator<Item = char>> = Box::new("hello".chars());
        let first2: String = dynit.by_ref().take(2).collect();
        let rest: String = dynit.collect();
        println!("{} {}", first2, rest);
        let mut pk = (Box::new(1..5) as Box<dyn Iterator<Item = i32>>).peekable();
        while let Some(x) = pk.next() { if let Some(&n) = pk.peek() { print!("{}<{} ", x, n); } }
        println!();
        let its: Vec<Box<dyn Iterator<Item = u8>>> = vec![Box::new(0..2), Box::new(vec![7, 8].into_iter()), Box::new(std::iter::once(9))];
        let all: Vec<u8> = its.into_iter().flatten().collect();
        println!("{:?}", all);
        let de: Box<dyn DoubleEndedIterator<Item = i32>> = Box::new(vec![1, 2, 3].into_iter());
        println!("{:?}", de.rev().collect::<Vec<_>>());
        let ex: &mut dyn ExactSizeIterator<Item = i32> = &mut vec![4, 5].into_iter();
        println!("{} {:?}", ex.len(), ex.next());
    }
    trait RevSafe: Iterator + Sized { fn rev_safe(self) -> std::vec::IntoIter<Self::Item> { let mut v: Vec<_> = self.collect(); v.reverse(); v.into_iter() } }
    impl<I: Iterator> RevSafe for I {}
}

mod y02 {
    use std::collections::HashMap;
    trait Plugin { fn name(&self) -> &str; fn handle(&mut self, ev: &Event) -> Option<String>; }
    #[derive(Debug, Clone)]
    enum Event { Key(char), Click { x: i32, y: i32 }, Tick(u64) }
    struct KeyLogger { keys: String }
    struct ClickCounter { by_quadrant: HashMap<(bool, bool), u32> }
    struct Ticker<F: FnMut(u64) -> bool> { pred: F, hits: Vec<u64> }
    impl Plugin for KeyLogger {
        fn name(&self) -> &str { "keys" }
        fn handle(&mut self, ev: &Event) -> Option<String> {
            if let Event::Key(c) = ev { self.keys.push(*c); Some(format!("typed {:?}", self.keys)) } else { None }
        }
    }
    impl Plugin for ClickCounter {
        fn name(&self) -> &str { "clicks" }
        fn handle(&mut self, ev: &Event) -> Option<String> {
            match *ev {
                Event::Click { x, y } => { let c = self.by_quadrant.entry((x >= 0, y >= 0)).or_insert(0); *c += 1; Some(format!("q{:?}={}", (x >= 0, y >= 0), c)) }
                _ => None,
            }
        }
    }
    impl<F: FnMut(u64) -> bool> Plugin for Ticker<F> {
        fn name(&self) -> &str { "ticker" }
        fn handle(&mut self, ev: &Event) -> Option<String> {
            match ev { Event::Tick(t) if (self.pred)(*t) => { self.hits.push(*t); Some(format!("tick hits {:?}", self.hits)) } _ => None }
        }
    }
    pub fn main() {
        let mut seen = 0;
        let mut plugins: Vec<Box<dyn Plugin + '_>> = vec![
            Box::new(KeyLogger { keys: String::new() }),
            Box::new(ClickCounter { by_quadrant: HashMap::new() }),
            Box::new(Ticker { pred: |t| { seen += 1; t % 3 == 0 }, hits: vec![] }),
        ];
        let evs = vec![Event::Key('a'), Event::Click { x: 1, y: -1 }, Event::Tick(3), Event::Tick(4), Event::Key('b'), Event::Click { x: 2, y: -5 }, Event::Tick(9)];
        for ev in &evs {
            for p in plugins.iter_mut() {
                if let Some(msg) = p.handle(ev) { println!("[{}] {:?} -> {}", p.name(), ev, msg); }
            }
        }
        let names: Vec<&str> = plugins.iter().map(|p| p.name()).collect();
        println!("{:?}", names);
        drop(plugins);
        println!("seen={}", seen);
    }
}

mod y03 {
    use std::ops::{Deref, DerefMut};
    struct Guarded<T> { val: T, log: Vec<&'static str> }
    impl<T> Deref for Guarded<T> { type Target = T; fn deref(&self) -> &T { &self.val } }
    impl<T> DerefMut for Guarded<T> { fn deref_mut(&mut self) -> &mut T { self.log.push("mut"); &mut self.val } }
    struct Outer { g: Guarded<Vec<String>> }
    fn takes_slice(s: &[String]) -> usize { s.len() }
    fn takes_str(s: &str) -> usize { s.len() }
    pub fn main() {
        let mut o = Outer { g: Guarded { val: vec!["a".into()], log: vec![] } };
        o.g.push("bb".into());
        o.g[0].push('!');
        let n = takes_slice(&o.g);
        let m = takes_str(&o.g[1]);
        println!("{} {} {:?} {:?}", n, m, *o.g, o.g.log);
        for s in o.g.iter_mut() { s.make_ascii_uppercase(); }
        println!("{:?} {}", o.g.val, o.g.log.len());
        let b: Box<Guarded<String>> = Box::new(Guarded { val: "hey".into(), log: vec![] });
        println!("{} {}", b.len(), b.to_uppercase());
        let rc = std::rc::Rc::new(Guarded { val: 5, log: vec![] });
        println!("{}", **rc + 1);
        let s = String::from("xyz");
        let r = &&&s;
        println!("{} {}", r.len(), r.chars().rev().collect::<String>());
    }
}

mod y04 {
    #[derive(Debug, Clone, PartialEq)]
    enum Expr { Num(f64), Var(String), Bin(Box<Expr>, char, Box<Expr>), Let(String, Box<Expr>, Box<Expr>), If(Box<Expr>, Box<Expr>, Box<Expr>) }
    use Expr::*;
    fn eval(e: &Expr, env: &mut Vec<(String, f64)>) -> Result<f64, String> {
        Ok(match e {
            Num(n) => *n,
            Var(v) => env.iter().rev().find(|(k, _)| k == v).map(|(_, x)| *x).ok_or_else(|| format!("unbound {}", v))?,
            Bin(a, op, b) => {
                let (x, y) = (eval(a, env)?, eval(b, env)?);
                match op { '+' => x + y, '-' => x - y, '*' => x * y, '/' if y == 0.0 => return Err("div0".into()), '/' => x / y, '<' => (x < y) as i32 as f64, _ => return Err(format!("op {}", op)) }
            }
            Let(n, v, body) => { let val = eval(v, env)?; env.push((n.clone(), val)); let r = eval(body, env); env.pop(); r? }
            If(c, t, f) => if eval(c, env)? != 0.0 { eval(t, env)? } else { eval(f, env)? },
        })
    }
    fn simplify(e: Expr) -> Expr {
        match e {
            Bin(a, op, b) => match (simplify(*a), op, simplify(*b)) {
                (Num(x), '+', Num(y)) => Num(x + y),
                (Num(x), '*', Num(y)) => Num(x * y),
                (e, '*', Num(o)) | (Num(o), '*', e) if o == 1.0 => e,
                (_, '*', Num(z)) | (Num(z), '*', _) if z == 0.0 => Num(0.0),
                (e, '+', Num(z)) | (Num(z), '+', e) if z == 0.0 => e,
                (a, op, b) => Bin(Box::new(a), op, Box::new(b)),
            },
            Let(n, v, b) => Let(n, Box::new(simplify(*v)), Box::new(simplify(*b))),
            other => other,
        }
    }
    fn b(a: Expr, op: char, c: Expr) -> Expr { Bin(Box::new(a), op, Box::new(c)) }
    pub fn main() {
        let prog = Let("x".into(), Box::new(b(Num(2.0), '+', Num(3.0))), Box::new(If(Box::new(b(Var("x".into()), '<', Num(10.0))), Box::new(b(Var("x".into()), '*', Var("x".into()))), Box::new(Num(-1.0)))));
        let mut env = vec![];
        println!("{:?}", eval(&prog, &mut env));
        println!("{:?}", eval(&b(Num(1.0), '/', Num(0.0)), &mut env));
        println!("{:?}", eval(&Var("y".into()), &mut env));
        let s = simplify(b(b(Var("a".into()), '*', Num(1.0)), '+', b(Num(0.0), '*', Var("q".into()))));
        println!("{:?}", s);
        let s2 = simplify(b(b(Num(2.0), '+', Num(3.0)), '*', b(Var("z".into()), '+', Num(0.0))));
        println!("{:?}", s2);
        println!("{}", s2 == b(Num(5.0), '*', Var("z".into())));
    }
}

fn main() {
    println!("== y01");
    y01::main();
    println!("== y02");
    y02::main();
    println!("== y03");
    y03::main();
    println!("== y04");
    y04::main();
}
