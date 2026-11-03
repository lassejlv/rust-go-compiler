// Moving out of boxes: `*b` whole moves, destructuring `let S { .. } = *b`,
// partial field moves (remaining fields must still drop), `self: Box<Self>`
// methods on trait objects, Box<dyn FnOnce> consumption, recursive boxed
// enums unwrapped by value, and Option<Box<T>>::map(|b| *b).
use std::cell::RefCell;

thread_local! { static EV: RefCell<Vec<String>> = RefCell::new(vec![]); }
fn ev(s: String) { EV.with(|e| e.borrow_mut().push(s)); }
fn dump(t: &str) { let v = EV.with(|e| std::mem::take(&mut *e.borrow_mut())); println!("{:<18}{}", t, v.join(" ")); }

#[derive(Debug)]
struct D(&'static str);
impl Drop for D { fn drop(&mut self) { ev(format!("~{}", self.0)); } }

struct Rec { a: D, b: D, c: D }

trait Stage { fn run(self: Box<Self>, input: i32) -> (i32, Option<Box<dyn Stage>>); fn label(&self) -> String; }
struct Add(i32, D);
struct Mul(i32, D, Option<Box<dyn Stage>>);
impl Stage for Add {
    fn run(self: Box<Self>, input: i32) -> (i32, Option<Box<dyn Stage>>) { (input + self.0, None) }
    fn label(&self) -> String { format!("add{}", self.0) }
}
impl Stage for Mul {
    fn run(self: Box<Self>, input: i32) -> (i32, Option<Box<dyn Stage>>) {
        let Mul(k, guard, next) = *self;
        ev(format!("mul uses {}", guard.0));
        (input * k, next)
    }
    fn label(&self) -> String { format!("mul{}", self.0) }
}

enum List { Cons(D, Box<List>), Nil }

fn main() {
    {
        let b = Box::new(D("whole"));
        let d = *b;
        ev(format!("moved {}", d.0));
    }
    dump("whole move:");
    {
        let b = Box::new(Rec { a: D("ra"), b: D("rb"), c: D("rc") });
        let Rec { a, c, .. } = *b;
        ev(format!("destructured {} {}", a.0, c.0));
    }
    dump("destructure:");
    {
        let b = Box::new(Rec { a: D("pa"), b: D("pb"), c: D("pc") });
        let x = b.b;
        ev(format!("took {}", x.0));
    }
    dump("partial field:");
    {
        let mut b = Box::new(Rec { a: D("ma"), b: D("mb"), c: D("mc") });
        let old = std::mem::replace(&mut b.a, D("ma2"));
        ev(format!("replaced {}", old.0));
        b.c = D("mc2");
        ev("assigned c".into());
    }
    dump("box field mut:");
    {
        let pipeline: Box<dyn Stage> = Box::new(Mul(3, D("g1"), Some(Box::new(Mul(2, D("g2"), Some(Box::new(Add(5, D("g3")))))))));
        let mut cur = Some(pipeline);
        let mut val = 1;
        while let Some(stage) = cur.take() {
            let l = stage.label();
            let (v, next) = stage.run(val);
            ev(format!("{} -> {}", l, v));
            val = v;
            cur = next;
        }
    }
    dump("self: Box<Self>:");
    {
        let s = D("captured");
        let f: Box<dyn FnOnce() -> String> = Box::new(move || format!("ran with {}", s.0));
        ev("before call".into());
        let r = f();
        ev(r);
        let g: Box<dyn FnOnce() -> usize> = Box::new({ let t = D("never-called"); move || t.0.len() });
        drop(g);
    }
    dump("Box<dyn FnOnce>:");
    {
        let mut l = List::Cons(D("n1"), Box::new(List::Cons(D("n2"), Box::new(List::Cons(D("n3"), Box::new(List::Nil))))));
        let mut names = vec![];
        loop {
            match l {
                List::Cons(d, rest) => { names.push(d.0); l = *rest; }
                List::Nil => break,
            }
        }
        ev(format!("walked {:?}", names));
    }
    dump("list by value:");
    {
        let o: Option<Box<D>> = Some(Box::new(D("optbox")));
        let inner: Option<D> = o.map(|b| *b);
        ev(format!("unboxed {:?}", inner.as_ref().map(|d| d.0)));
        let v: Vec<Box<D>> = vec![Box::new(D("vb0")), Box::new(D("vb1"))];
        let unboxed: Vec<D> = v.into_iter().map(|b| *b).rev().collect();
        ev(format!("vec unboxed {}", unboxed.len()));
    }
    dump("map unbox:");
}
