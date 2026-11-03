// Partial moves through patterns: moving some fields out of enum payloads,
// Option<struct>, nested tuples and arrays inside match/if-let/let-else,
// mixing by-value and `ref` bindings; the un-moved remainder must be dropped
// at the end of the owner's scope (not earlier, not never).
use std::cell::RefCell;

thread_local! { static EV: RefCell<Vec<String>> = RefCell::new(vec![]); }
fn ev(s: String) { EV.with(|e| e.borrow_mut().push(s)); }
fn dump(t: &str) { let v = EV.with(|e| std::mem::take(&mut *e.borrow_mut())); println!("{:<16}{}", t, v.join(" ")); }

#[derive(Debug)]
struct D(&'static str);
impl Drop for D { fn drop(&mut self) { ev(format!("~{}", self.0)); } }

struct Pair { left: D, right: D }
enum Msg { Two(D, D), Named { head: D, tail: D, n: u8 }, Nested(Option<Pair>), Empty }

fn main() {
    {
        let m = Msg::Two(D("t1"), D("t2"));
        if let Msg::Two(a, _) = m {
            ev(format!("took {}", a.0));
        }
        ev("after if-let".into());
    }
    dump("enum tuple:");
    {
        let m = Msg::Named { head: D("h"), tail: D("t"), n: 3 };
        match m {
            Msg::Named { tail, n, .. } => ev(format!("tail {} n {}", tail.0, n)),
            _ => {}
        }
        ev("after match".into());
    }
    dump("enum struct:");
    {
        let m = Msg::Nested(Some(Pair { left: D("nl"), right: D("nr") }));
        if let Msg::Nested(Some(Pair { right, .. })) = m {
            ev(format!("right {}", right.0));
            drop(right);
            ev("dropped right".into());
        }
        ev("end scope".into());
    }
    dump("nested option:");
    {
        let o = Some(Pair { left: D("ol"), right: D("or") });
        let Some(Pair { left, ref right }) = o else { return };
        ev(format!("left {} ref right {}", left.0, right.0));
    }
    dump("let-else mix:");
    {
        let t = ((D("a0"), D("a1")), D("b"), [D("c0"), D("c1")]);
        let ((x, _), _, [_, y]) = t;
        ev(format!("x {} y {}", x.0, y.0));
    }
    dump("nested tuple:");
    {
        let v = vec![Msg::Two(D("v1"), D("v2")), Msg::Empty, Msg::Named { head: D("vh"), tail: D("vt"), n: 1 }];
        for m in v {
            match m {
                Msg::Two(_, keep) => ev(format!("keep {}", keep.0)),
                Msg::Named { head, .. } => ev(format!("head {}", head.0)),
                _ => ev("other".into()),
            }
        }
        ev("loop end".into());
    }
    dump("loop moves:");
    {
        let p = Pair { left: D("pl"), right: D("pr") };
        let Pair { left: moved, .. } = p;
        ev(format!("moved {}", moved.0));
        let r = &p.right;
        ev(format!("still borrow {}", r.0));
    }
    dump("struct remains:");
    {
        let o: Option<(D, D)> = Some((D("q0"), D("q1")));
        match o {
            Some((first, ref second)) if second.0 == "q1" => ev(format!("guarded {} {}", first.0, second.0)),
            Some(_) => ev("other some".into()),
            None => ev("none".into()),
        }
        ev("after guard match".into());
    }
    dump("guard + move:");
    {
        let opt = Some(Pair { left: D("ml"), right: D("mr") });
        let left_only = opt.map(|p| p.left);
        ev(format!("mapped {:?}", left_only.as_ref().map(|d| d.0)));
    }
    dump("closure move:");
}
