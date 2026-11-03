use std::rc::{Rc, Weak};
use std::sync::Arc;

struct D(&'static str);
impl Drop for D {
    fn drop(&mut self) { println!("drop {}", self.0); }
}

fn main() {
    let rc = Rc::new(D("rc"));
    let rc2 = rc.clone();
    let raw = Rc::into_raw(rc);
    println!("count after into_raw {}", Rc::strong_count(&rc2));
    unsafe {
        println!("via raw {}", (*raw).0);
        Rc::increment_strong_count(raw);
        println!("count {}", Rc::strong_count(&rc2));
        Rc::decrement_strong_count(raw);
        let back = Rc::from_raw(raw);
        println!("ptr_eq {}", Rc::ptr_eq(&back, &rc2));
    }
    println!("count end {}", Rc::strong_count(&rc2));
    drop(rc2);
    println!("--");
    let a = Arc::new(vec![1, 2, 3]);
    let p = Arc::into_raw(a.clone());
    let restored = unsafe { Arc::from_raw(p) };
    println!("{:?} {}", restored, Arc::strong_count(&a));
    drop(restored);
    println!("{}", Arc::strong_count(&a));
    let mut a2 = Arc::new(5);
    *Arc::get_mut(&mut a2).unwrap() += 1;
    let a3 = a2.clone();
    println!("get_mut shared {:?}", Arc::get_mut(&mut a2));
    *Arc::make_mut(&mut a2) += 100;
    println!("{} {}", a2, a3);
    println!("try_unwrap {:?}", Arc::try_unwrap(a3));
    let w: Weak<D>;
    {
        let s = Rc::new(D("weak target"));
        w = Rc::downgrade(&s);
        println!("upgrade {}", w.upgrade().map(|r| r.0).unwrap_or("none"));
        println!("weak_count {}", Rc::weak_count(&s));
    }
    println!("upgrade after {}", w.upgrade().is_none());
    let wr = Weak::<i32>::new();
    println!("empty weak {:?}", wr.upgrade());
    let cyc = Rc::new_cyclic(|me: &Weak<(i32, Weak<i32>)>| {
        let _ = me;
        (7, Weak::new())
    });
    println!("cyclic {}", cyc.0);
    let shared: Rc<[i32]> = Rc::from(vec![4, 5, 6]);
    println!("{:?} {}", shared, shared.len());
    let sstr: Arc<str> = Arc::from("arc str");
    println!("{}", sstr.to_uppercase());
    let u = Rc::new(String::from("unwrap me"));
    println!("{:?}", Rc::into_inner(u));
    let pinned = Rc::pin(D("pinned"));
    println!("pinned {}", pinned.0);
}
