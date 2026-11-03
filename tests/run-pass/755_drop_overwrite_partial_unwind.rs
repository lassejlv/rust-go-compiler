// Drop semantics on overwrite and partial moves, plus drops of partially
// constructed aggregates when a panic interrupts construction.
use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
use std::panic;

thread_local! { static EV: RefCell<Vec<String>> = RefCell::new(vec![]); }
fn ev(s: String) { EV.with(|e| e.borrow_mut().push(s)); }
fn dump(t: &str) { let v = EV.with(|e| std::mem::take(&mut *e.borrow_mut())); println!("{:<20}{}", t, v.join(" ")); }

#[derive(Debug)]
struct D(&'static str);
impl Drop for D { fn drop(&mut self) { ev(format!("~{}", self.0)); } }

#[derive(Debug)]
enum E { A(D), B(D, D), C }

struct S { x: D, y: D, z: D }

fn boom(tag: &'static str) -> D { if tag.len() < 100 { panic!("boom {}", tag) } D(tag) }

fn main() {
    panic::set_hook(Box::new(|_| {}));
    {
        let mut e = E::B(D("b1"), D("b2"));
        ev("assign A".into());
        e = E::A(D("a1"));
        ev("assign C".into());
        e = E::C;
        ev(format!("now {:?}", e));
        let old = std::mem::replace(&mut e, E::A(D("a2")));
        ev(format!("replaced {:?}", old));
    }
    dump("enum overwrite:");
    {
        let mut o: Option<Box<dyn std::fmt::Debug>> = Some(Box::new(D("boxed1")));
        o = Some(Box::new(D("boxed2")));
        ev("set2".into());
        o = None;
        ev(format!("none {}", o.is_none()));
    }
    dump("option box:");
    {
        let mut v = vec![D("v0"), D("v1"), D("v2")];
        v[1] = D("v1new");
        ev("indexed assign".into());
        v.clear();
        ev("cleared".into());
        v.push(D("v3"));
    }
    dump("vec ops:");
    {
        let mut m = HashMap::new();
        m.insert(1, D("h1"));
        let old = m.insert(1, D("h1b"));
        ev(format!("old {:?}", old.as_ref().map(|d| d.0)));
        drop(old);
        let mut b = BTreeMap::new();
        b.insert("k", D("t1"));
        b.insert("k", D("t2"));
        ev("btree overwrite".into());
        b.remove("k");
        ev("removed".into());
    }
    dump("map overwrite:");
    {
        let arr = [D("r0"), D("r1"), D("r2")];
        let [a, _, c] = arr;
        ev(format!("got {} {}", a.0, c.0));
        drop(c);
    }
    dump("array partial:");
    {
        let t = (D("t0"), D("t1"), D("t2"));
        let (x, _) = (t.0, 5);
        ev(format!("x {}", x.0));
        let rest = t.2;
        ev(format!("rest {}", rest.0));
    }
    dump("tuple partial:");
    {
        let s = S { x: D("sx"), y: D("sy"), z: D("sz") };
        let S { y, .. } = s;
        ev(format!("y {}", y.0));
    }
    dump("struct partial:");
    {
        let b = Box::new(D("inbox"));
        let inner = *b;
        ev(format!("unboxed {}", inner.0));
        let bs = Box::new(S { x: D("bx"), y: D("by"), z: D("bz") });
        let moved_x = bs.x;
        ev(format!("moved {}", moved_x.0));
    }
    dump("box moves:");
    {
        let r = panic::catch_unwind(|| { let _arr = [D("p0"), D("p1"), boom("p2"), D("p3")]; });
        ev(format!("caught {}", r.is_err()));
    }
    dump("array unwind:");
    {
        let r = panic::catch_unwind(|| { let _s = S { x: D("fx"), y: boom("fy"), z: D("fz") }; });
        ev(format!("caught {}", r.is_err()));
    }
    dump("struct unwind:");
    {
        let r = panic::catch_unwind(|| { let _v = vec![D("vv0"), D("vv1"), boom("vv2")]; });
        ev(format!("caught {}", r.is_err()));
    }
    dump("vec! unwind:");
    {
        let r = panic::catch_unwind(|| { let _t = (D("tt0"), (D("tt1"), boom("tt2")), D("tt3")); });
        ev(format!("caught {}", r.is_err()));
    }
    dump("tuple unwind:");
    {
        let r = panic::catch_unwind(|| {
            let mut v: Vec<D> = Vec::new();
            v.push(D("loop0"));
            for name in ["loop1", "loop2"] { v.push(D(name)); }
            let _extra = D("extra");
            v.push(boom("loop3"));
        });
        ev(format!("caught {}", r.is_err()));
    }
    dump("vec push unwind:");
    {
        let r = panic::catch_unwind(|| {
            let e = E::B(D("eb1"), boom("eb2"));
            drop(e);
        });
        ev(format!("caught {}", r.is_err()));
    }
    dump("enum ctor unwind:");
}
