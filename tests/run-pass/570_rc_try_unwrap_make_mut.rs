// Rc::try_unwrap / into_inner / make_mut / get_mut / ptr_eq / Weak upgrade semantics, Rc<str>, Rc<[T]>, cyclic
use std::cell::RefCell;
use std::rc::{Rc, Weak};

#[derive(Debug, Clone)]
struct Data {
    v: Vec<i32>,
}
impl Drop for Data {
    fn drop(&mut self) {
        println!("drop Data{:?}", self.v);
    }
}

struct Owner {
    name: String,
    gadgets: RefCell<Vec<Weak<Gadget>>>,
}
struct Gadget {
    id: u32,
    owner: Rc<Owner>,
}

struct SelfRef {
    me: Weak<SelfRef>,
    val: i32,
}

fn main() {
    let a = Rc::new(Data { v: vec![1] });
    let b = Rc::clone(&a);
    let a = match Rc::try_unwrap(a) {
        Ok(d) => {
            println!("unwrapped {:?}", d.v);
            Rc::new(d)
        }
        Err(rc) => {
            println!("still shared: count={}", Rc::strong_count(&rc));
            rc
        }
    };
    drop(b);
    let d = Rc::try_unwrap(a).ok().unwrap();
    println!("now unique {:?}", d.v);
    drop(d);

    let mut x = Rc::new(Data { v: vec![10, 20] });
    let y = x.clone();
    Rc::make_mut(&mut x).v.push(30);
    println!("x={:?} y={:?} ptr_eq={}", x.v, y.v, Rc::ptr_eq(&x, &y));
    Rc::make_mut(&mut x).v.push(40);
    println!("x={:?} count={}", x.v, Rc::strong_count(&x));
    println!("get_mut on shared y: {}", Rc::get_mut(&mut y.clone()).is_none());
    let mut z = Rc::new(5);
    *Rc::get_mut(&mut z).unwrap() += 1;
    println!("{}", z);
    drop(y);
    println!("into_inner {:?}", Rc::into_inner(x).map(|d| d.v.len()));

    let owner = Rc::new(Owner { name: "Gizmo".into(), gadgets: RefCell::new(vec![]) });
    let g1 = Rc::new(Gadget { id: 1, owner: owner.clone() });
    let g2 = Rc::new(Gadget { id: 2, owner: owner.clone() });
    owner.gadgets.borrow_mut().push(Rc::downgrade(&g1));
    owner.gadgets.borrow_mut().push(Rc::downgrade(&g2));
    for g in owner.gadgets.borrow().iter() {
        if let Some(g) = g.upgrade() {
            println!("gadget {} owned by {}", g.id, g.owner.name);
        }
    }
    drop(g2);
    let alive: Vec<u32> = owner.gadgets.borrow().iter().filter_map(|w| w.upgrade()).map(|g| g.id).collect();
    println!("alive {:?} owner strong={} weak={}", alive, Rc::strong_count(&owner), Rc::weak_count(&owner));
    let w = Rc::downgrade(&g1);
    println!("weak strong_count={} weak_count={}", w.strong_count(), w.weak_count());
    drop(g1);
    println!("after drop: upgrade={} strong_count={}", w.upgrade().is_none(), w.strong_count());
    let empty: Weak<i32> = Weak::new();
    println!("{:?} {}", empty.upgrade(), empty.strong_count());

    let s: Rc<str> = Rc::from("shared text");
    let s2 = s.clone();
    println!("{} {} {}", &s[..6], s2.len(), Rc::strong_count(&s));
    let sl: Rc<[i32]> = vec![1, 2, 3].into();
    let sl2: Rc<[i32]> = Rc::from(&[4, 5][..]);
    println!("{:?} {:?} {}", sl, sl2, sl.iter().chain(sl2.iter()).sum::<i32>());
    let cyc = Rc::new_cyclic(|me| SelfRef { me: me.clone(), val: 99 });
    println!("{} {}", cyc.me.upgrade().unwrap().val, Rc::weak_count(&cyc));
    let list: Vec<Rc<Data>> = (0..3).map(|i| Rc::new(Data { v: vec![i] })).collect();
    let shared_list: Vec<Rc<Data>> = list.iter().cloned().rev().collect();
    println!("{:?}", shared_list.iter().map(|d| Rc::strong_count(d)).collect::<Vec<_>>());
    drop(list);
    println!("list dropped");
    let unique: Vec<Data> = shared_list.into_iter().filter_map(|r| Rc::try_unwrap(r).ok()).collect();
    println!("{:?}", unique.iter().map(|d| d.v[0]).collect::<Vec<_>>());
    let rc_default: Rc<Vec<u8>> = Rc::default();
    println!("{:?} {}", rc_default, Rc::unwrap_or_clone(Rc::new(String::from("uoc"))));
    let p1 = Rc::new(1);
    let p2 = Rc::new(1);
    println!("{} {} {}", p1 == p2, Rc::ptr_eq(&p1, &p2), p1 < Rc::new(2));
    println!("end");
}
