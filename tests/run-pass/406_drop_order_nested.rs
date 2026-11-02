// Drop order for nested structs, enums, tuples, arrays, Vec, Box and Option;
// locals drop in reverse declaration order, fields in declaration order.
use std::cell::RefCell;

thread_local! {
    static LOG: RefCell<Vec<String>> = RefCell::new(Vec::new());
}

fn log(s: String) {
    LOG.with(|l| l.borrow_mut().push(s));
}

fn flush(label: &str) {
    LOG.with(|l| {
        let v: Vec<String> = l.borrow_mut().drain(..).collect();
        println!("{}: {}", label, v.join(" "));
    });
}

struct D(&'static str);
impl Drop for D {
    fn drop(&mut self) {
        log(self.0.to_string());
    }
}

struct Pair {
    a: D,
    b: D,
}
impl Drop for Pair {
    fn drop(&mut self) {
        log(format!("Pair({},{})", self.a.0, self.b.0));
    }
}

struct Outer {
    first: Pair,
    middle: (D, D),
    last: [D; 2],
}

enum E {
    One(D),
    Two(D, D),
    Named { x: D, y: Box<D> },
    Empty,
}

fn make(tag: &'static str) -> D {
    log(format!("make {}", tag));
    D(tag)
}

fn main() {
    {
        let _a = D("a");
        let _b = D("b");
        let _c = D("c");
    }
    flush("locals");

    {
        let _o = Outer {
            first: Pair { a: D("p.a"), b: D("p.b") },
            middle: (D("t0"), D("t1")),
            last: [D("arr0"), D("arr1")],
        };
    }
    flush("outer");

    {
        let v = vec![E::One(D("e1")), E::Empty, E::Two(D("e2a"), D("e2b")), E::Named { x: D("nx"), y: Box::new(D("ny")) }];
        log(format!("vec len {}", v.len()));
    }
    flush("enums");

    {
        // struct literal fields evaluated in source order, but stored by field order
        let _p = Pair { b: make("B"), a: make("A") };
    }
    flush("literal order");

    {
        let mut x = Some(D("opt1"));
        x = None;
        log(format!("after reassign none={}", x.is_none()));
        x = Some(D("opt2"));
        let _ = x;
        log("end scope".to_string());
    }
    flush("option");

    {
        let t = (D("t.0"), (D("t.1.0"), D("t.1.1")), D("t.2"));
        let _keep = t.1;
        log("moved t.1".to_string());
    }
    flush("tuple partial");

    {
        let mut v = Vec::new();
        for name in ["v0", "v1", "v2", "v3"] {
            v.push(D(name));
        }
        v.remove(1);
        log("removed".to_string());
        v.truncate(2);
        log("truncated".to_string());
        let last = v.pop();
        log(format!("popped {}", last.as_ref().map(|d| d.0).unwrap_or("-")));
    }
    flush("vec ops");

    {
        let _shadow = D("first");
        let _shadow = D("second");
        log("shadowed".to_string());
    }
    flush("shadow");

    {
        let b: Box<[D]> = vec![D("bs0"), D("bs1"), D("bs2")].into_boxed_slice();
        drop(b);
        log("after drop".to_string());
    }
    flush("boxed slice");
}
