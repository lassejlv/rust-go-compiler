// When do closure captures get dropped? Boxed FnMut in a Vec (dropped with
// the Vec, on remove, on replacement), move closures capturing disjoint
// fields of a struct (edition 2021 precise capture) vs whole-struct capture,
// FnOnce consumed by call, closures returned from functions, and captured
// references vs captured values.
use std::cell::RefCell;

thread_local! { static EV: RefCell<Vec<String>> = RefCell::new(vec![]); }
fn ev(s: String) { EV.with(|e| e.borrow_mut().push(s)); }
fn dump(t: &str) { let v = EV.with(|e| std::mem::take(&mut *e.borrow_mut())); println!("{:<18}{}", t, v.join(" ")); }

#[derive(Debug)]
struct D(&'static str);
impl Drop for D { fn drop(&mut self) { ev(format!("~{}", self.0)); } }

struct Pair { left: D, right: D, n: u32 }
struct WithDrop { a: D, b: D }
impl Drop for WithDrop { fn drop(&mut self) { ev("~WithDrop".into()); } }

fn make_counter(tag: &'static str) -> impl FnMut() -> u32 {
    let guard = D(tag);
    let mut n = 0;
    move || { n += 1; let _ = &guard; n }
}

fn main() {
    {
        let mut fs: Vec<Box<dyn FnMut() -> String>> = Vec::new();
        for name in ["c0", "c1", "c2"] {
            let d = D(name);
            let mut calls = 0;
            fs.push(Box::new(move || { calls += 1; format!("{}#{}", d.0, calls) }));
        }
        ev(format!("{} {}", fs[0](), fs[0]()));
        let removed = fs.remove(1);
        ev("removed".into());
        drop(removed);
        fs[0] = Box::new(|| "replacement".to_string());
        ev(format!("{}", fs[0]()));
        ev("vec end".into());
    }
    dump("vec of FnMut:");
    {
        let p = Pair { left: D("left"), right: D("right"), n: 7 };
        let c = move || p.left.0.len() as u32 + p.n;
        ev(format!("call {}", c()));
        drop(c);
        ev("closure dropped".into());
        ev(format!("p.right still here {}", p.right.0));
    }
    dump("disjoint move:");
    {
        let w = WithDrop { a: D("wa"), b: D("wb") };
        let c = move || w.a.0.len();
        ev(format!("call {}", c()));
        drop(c);
        ev("after drop c".into());
    }
    dump("Drop struct cap:");
    {
        let d = D("once");
        let f = move || { let inner = d; format!("consumed {}", inner.0) };
        ev("before".into());
        ev(f());
        ev("after".into());
    }
    dump("FnOnce consume:");
    {
        let d = D("by-ref");
        let r = || d.0.len();
        ev(format!("len {}", r()));
        drop(r);
        ev("ref closure gone".into());
    }
    dump("ref capture:");
    {
        let mut c1 = make_counter("ctr");
        ev(format!("{} {} {}", c1(), c1(), c1()));
        let c2 = make_counter("ctr2");
        drop(c2);
        ev("c2 dropped".into());
    }
    dump("returned closure:");
    {
        let ds = vec![D("v0"), D("v1")];
        let total = move || ds.len();
        let boxed: Box<dyn Fn() -> usize> = Box::new(total);
        ev(format!("len {}", boxed()));
        let opt: Option<Box<dyn Fn() -> usize>> = Some(boxed);
        ev("moved into option".into());
        drop(opt);
        ev("option dropped".into());
    }
    dump("box in option:");
    {
        let a = D("tuple-a");
        let b = D("tuple-b");
        let t = (a, b);
        let c = move || t.1 .0.len();
        ev(format!("{}", c()));
    }
    dump("tuple field cap:");
    {
        let mut log = Vec::new();
        let d = D("mut-capture");
        let mut push = |s: &str| log.push(format!("{}:{}", s, d.0));
        push("x");
        push("y");
        ev(format!("{:?}", log));
    }
    dump("FnMut by ref:");
}
