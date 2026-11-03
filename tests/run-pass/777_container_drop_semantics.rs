// Which elements get dropped, and when, for container operations:
// partially consumed vec::IntoIter, Drain dropped early, retain/dedup/
// truncate/split_off/clear, VecDeque wraparound drops, BTreeMap/BTreeSet
// into_iter partially consumed, iterator adaptors (take/skip/filter) over
// owned items, mem::take on Vec, Option::replace, and array::IntoIter.
use std::cell::RefCell;
use std::collections::{BTreeMap, VecDeque};

thread_local! { static EV: RefCell<Vec<String>> = RefCell::new(vec![]); }
fn ev(s: String) { EV.with(|e| e.borrow_mut().push(s)); }
fn dump(t: &str) { let v = EV.with(|e| std::mem::take(&mut *e.borrow_mut())); println!("{:<16}{}", t, v.join(" ")); }

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
struct D(u32);
impl Drop for D { fn drop(&mut self) { ev(format!("~{}", self.0)); } }
fn ds(r: std::ops::Range<u32>) -> Vec<D> { r.map(D).collect() }

fn main() {
    {
        let mut it = ds(0..5).into_iter();
        let a = it.next();
        let b = it.next_back();
        ev(format!("got {:?} {:?}", a.as_ref().map(|d| d.0), b.as_ref().map(|d| d.0)));
        drop(it);
        ev("iter dropped".into());
    }
    dump("IntoIter:");
    {
        let mut v = ds(10..16);
        {
            let mut dr = v.drain(1..4);
            let x = dr.next();
            ev(format!("drained one {:?}", x.map(|d| d.0)));
        }
        ev(format!("left {:?}", v.iter().map(|d| d.0).collect::<Vec<_>>()));
    }
    dump("Drain early:");
    {
        let mut v = ds(20..26);
        v.retain(|d| d.0 % 2 == 0);
        ev("after retain".into());
        v.truncate(1);
        ev("after truncate".into());
        let mut w = vec![D(1), D(1), D(2), D(2), D(2), D(3)];
        w.dedup();
        ev(format!("dedup -> {:?}", w.iter().map(|d| d.0).collect::<Vec<_>>()));
        let tail = w.split_off(1);
        ev(format!("split_off tail len {}", tail.len()));
        drop(tail);
        w.clear();
        ev("cleared".into());
    }
    dump("vec ops:");
    {
        let mut dq: VecDeque<D> = VecDeque::with_capacity(4);
        for i in 30..34 { dq.push_back(D(i)); }
        dq.pop_front();
        dq.pop_front();
        dq.push_back(D(34));
        dq.push_back(D(35));
        ev(format!("deque {:?}", dq.iter().map(|d| d.0).collect::<Vec<_>>()));
        dq.truncate(3);
        ev("deque truncated".into());
    }
    dump("VecDeque:");
    {
        let m: BTreeMap<u32, D> = (40..45).map(|i| (i, D(i))).collect();
        let mut it = m.into_iter();
        let first = it.next().map(|(k, _)| k);
        ev(format!("first key {:?}", first));
        drop(it);
        ev("map iter dropped".into());
    }
    dump("BTreeMap iter:");
    {
        let v = ds(50..56);
        let kept: Vec<D> = v.into_iter().skip(1).filter(|d| d.0 % 2 == 1).take(2).collect();
        ev(format!("kept {:?}", kept.iter().map(|d| d.0).collect::<Vec<_>>()));
    }
    dump("adaptors:");
    {
        let mut v = ds(60..62);
        let old = std::mem::take(&mut v);
        ev(format!("taken {} left {}", old.len(), v.len()));
        drop(old);
        let mut o = Some(D(63));
        let prev = o.replace(D(64));
        ev(format!("replaced {:?}", prev.as_ref().map(|d| d.0)));
        drop(prev);
        ev("end".into());
    }
    dump("take/replace:");
    {
        let arr = [D(70), D(71), D(72), D(73)];
        let mut it = arr.into_iter();
        it.next();
        let last = it.next_back();
        ev(format!("last {:?} remaining {}", last.as_ref().map(|d| d.0), it.len()));
    }
    dump("array IntoIter:");
    {
        let mut v = ds(80..84);
        v.swap_remove(0);
        ev("swap_removed".into());
        v.insert(1, D(89));
        let s: Vec<D> = v.splice(1..3, [D(90), D(91), D(92)]).collect();
        ev(format!("spliced out {:?}", s.iter().map(|d| d.0).collect::<Vec<_>>()));
        drop(s);
        ev(format!("now {:?}", v.iter().map(|d| d.0).collect::<Vec<_>>()));
    }
    dump("swap/splice:");
    {
        let mut v = ds(100..104);
        v.sort_by(|a, b| b.cmp(a));
        let mx = v.iter().max().cloned();
        ev(format!("max clone {:?}", mx.as_ref().map(|d| d.0)));
        v.resize_with(6, || D(109));
        v.resize(2, D(999));
        ev("resized".into());
    }
    dump("sort/resize:");
}
