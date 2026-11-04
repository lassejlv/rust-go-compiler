// Arc::make_mut/get_mut/try_unwrap/into_inner/ptr_eq/downgrade and Rc analogues.
use std::rc::Rc;
use std::sync::{Arc, Weak};
use std::thread;

#[derive(Clone, Debug)]
struct Doc {
    lines: Vec<String>,
}
impl Drop for Doc {
    fn drop(&mut self) {
        println!("drop Doc({})", self.lines.len());
    }
}

fn main() {
    let mut a = Arc::new(Doc { lines: vec!["a".into()] });
    Arc::make_mut(&mut a).lines.push("b".into()); // unique: no clone
    println!("unique edit {:?}", a.lines);
    let b = Arc::clone(&a);
    Arc::make_mut(&mut a).lines.push("c".into()); // shared: clones
    println!("a={:?} b={:?} ptr_eq={}", a.lines, b.lines, Arc::ptr_eq(&a, &b));
    println!("get_mut on unique a: {}", Arc::get_mut(&mut a).is_some());
    let mut b2 = b.clone();
    println!("get_mut on shared b2: {}", Arc::get_mut(&mut b2).is_none());
    drop(b2);

    let w: Weak<Doc> = Arc::downgrade(&b);
    println!("strong={} weak={}", Arc::strong_count(&b), Arc::weak_count(&b));
    let b = match Arc::try_unwrap(b) {
        Ok(doc) => {
            println!("unwrapped {:?}", doc.lines);
            Arc::new(doc)
        }
        Err(arc) => arc,
    };
    println!("weak upgrade after unwrap: {}", w.upgrade().is_none());
    let c = b.clone();
    let r = Arc::try_unwrap(c);
    println!("try_unwrap shared is_err {}", r.is_err());
    let c = r.unwrap_err();
    let h = thread::spawn(move || Arc::into_inner(c).map(|d| d.lines.len()));
    let first = h.join().unwrap();
    let second = Arc::into_inner(b).map(|d| d.lines.len());
    println!("into_inner results: {:?} {:?}", first, second);

    // Arc<[T]> and Arc<str>
    let s: Arc<str> = Arc::from("shared str");
    let sl: Arc<[u32]> = vec![1, 2, 3].into();
    let s2 = s.clone();
    println!("{} {:?} {}", s2, sl, Arc::strong_count(&s));

    // Rc make_mut with weak pointers disassociates
    let mut r = Rc::new(vec![1, 2]);
    let wr = Rc::downgrade(&r);
    Rc::make_mut(&mut r).push(3);
    println!("rc {:?} weak alive {}", r, wr.upgrade().is_some());
    let r2 = r.clone();
    Rc::make_mut(&mut r).push(4);
    println!("rc {:?} r2 {:?}", r, r2);
    println!("rc unwrap_or_clone {:?}", Rc::unwrap_or_clone(r2));
    let arc_cyc = Arc::new_cyclic(|me: &Weak<Doc>| {
        println!("in new_cyclic, upgrade none: {}", me.upgrade().is_none());
        Doc { lines: vec!["cyc".into(); 3] }
    });
    println!("cyc {}", arc_cyc.lines.len());
    println!("end");
}
