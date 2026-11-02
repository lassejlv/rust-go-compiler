// Regression: `continue` drops the loop's pattern bindings; private fields of
// a smart pointer don't shadow fields of its target during autoderef.
use std::cell::RefCell;
use std::rc::Rc;

struct Node { value: i64, id: u32 }

mod wrap {
    pub struct Guard<'a> { value: &'a mut i64, pub tag: u8 }
    impl<'a> Guard<'a> {
        pub fn new(value: &'a mut i64) -> Self { Guard { value, tag: 7 } }
        pub fn get(&self) -> i64 { *self.value }
    }
    impl<'a> std::ops::Deref for Guard<'a> {
        type Target = i64;
        fn deref(&self) -> &i64 { self.value }
    }
}

struct Holder { value: String }
struct Outer { inner: Holder }
impl std::ops::Deref for Outer {
    type Target = Holder;
    fn deref(&self) -> &Holder { &self.inner }
}

fn main() {
    let x = Rc::new(RefCell::new(Node { value: 5, id: 1 }));
    let mut stack = vec![Rc::clone(&x), Rc::clone(&x), Rc::clone(&x)];
    let mut k = 0;
    while let Some(n) = stack.pop() {
        let nb = n.borrow();
        k += 1;
        if k < 3 { continue; }
        println!("last {} {}", nb.value, nb.id);
    }
    println!("while-let: count {} borrow_mut ok {}", Rc::strong_count(&x), x.try_borrow_mut().is_ok());

    let items = vec![Rc::clone(&x), Rc::clone(&x)];
    for it in items {
        if it.borrow().id == 1 { continue; }
    }
    println!("for: count {}", Rc::strong_count(&x));

    let mut j = 0;
    loop {
        j += 1;
        let tmp = Rc::clone(&x);
        if j < 4 { continue; }
        println!("loop: count inside {}", Rc::strong_count(&tmp));
        break;
    }
    println!("loop: count {}", Rc::strong_count(&x));

    // field access through Ref / RefMut reaches the target's fields
    x.borrow_mut().value += 10;
    let r = x.borrow();
    let v: i64 = r.value;
    println!("ref field {} {}", v, r.id);
    drop(r);
    {
        let mut m = x.borrow_mut();
        m.value *= 2;
    }
    println!("after mut {}", x.borrow().value);
    let sum: i64 = [&x, &x].iter().map(|n| n.try_borrow().map(|b| b.value).unwrap_or(0)).sum();
    println!("sum {}", sum);

    // private field of a local Deref type is skipped from outside its module
    let mut n = 41;
    let g = wrap::Guard::new(&mut n);
    println!("guard {} tag {} get {}", *g + 1, g.tag, g.get());

    // accessible fields still win at the first step
    let o = Outer { inner: Holder { value: "inner".to_string() } };
    println!("outer {}", o.value);
}
