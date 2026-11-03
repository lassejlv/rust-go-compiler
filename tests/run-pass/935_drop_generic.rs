use std::cell::RefCell;
use std::rc::Rc;
use std::fmt::Debug;

type Log = Rc<RefCell<Vec<String>>>;

struct Guard<T: Debug> { val: T, log: Log }
impl<T: Debug> Drop for Guard<T> {
    fn drop(&mut self) { self.log.borrow_mut().push(format!("drop {:?}", self.val)); }
}

struct Pair<A: Debug, B: Debug> { first: Guard<A>, second: Guard<B> }

fn g<T: Debug>(v: T, log: &Log) -> Guard<T> { Guard { val: v, log: log.clone() } }

fn consume<T: Debug>(g: Guard<T>) -> String { format!("consumed {:?}", g.val) }

fn make_and_pick<T: Debug + Clone>(a: T, b: T, log: &Log, first: bool) -> Guard<T> {
    let ga = g(a, log);
    let gb = g(b, log);
    if first { ga } else { gb }
}

enum Holder<T: Debug> { One(Guard<T>), Two(Guard<T>, Guard<T>), Empty }

fn main() {
    let log: Log = Rc::new(RefCell::new(vec![]));
    {
        let _a = g(1, &log);
        let _b = g("two", &log);
        let _p = Pair { first: g('x', &log), second: g(2.5, &log) };
        log.borrow_mut().push("end scope 1".into());
    }
    {
        let v = vec![g(10, &log), g(11, &log), g(12, &log)];
        let s = consume(g(99, &log));
        log.borrow_mut().push(s);
        drop(v);
        log.borrow_mut().push("after vec drop".into());
    }
    {
        let k = make_and_pick(5, 6, &log, true);
        log.borrow_mut().push(format!("kept {:?}", k.val));
        let k2 = make_and_pick("p", "q", &log, false);
        log.borrow_mut().push(format!("kept {:?}", k2.val));
    }
    {
        let mut h = Holder::Two(g(1u8, &log), g(2u8, &log));
        log.borrow_mut().push("reassign".into());
        h = Holder::One(g(3u8, &log));
        if let Holder::One(ref x) = h { log.borrow_mut().push(format!("holding {:?}", x.val)); }
        h = Holder::Empty;
        let _ = &h;
        log.borrow_mut().push("end holder".into());
    }
    {
        let _ = g("ignored", &log);
        log.borrow_mut().push("after underscore".into());
        let opt: Option<Guard<i32>> = Some(g(7, &log));
        let taken = opt.map(|x| x.val + 1);
        log.borrow_mut().push(format!("mapped {:?}", taken));
        let b: Box<Guard<Vec<i32>>> = Box::new(g(vec![1], &log));
        let rc1 = Rc::new(g("rc", &log));
        let rc2 = rc1.clone();
        drop(rc1);
        log.borrow_mut().push("rc1 dropped".into());
        drop(rc2);
        let _keep = b;
        log.borrow_mut().push("end last".into());
    }
    for l in log.borrow().iter() { println!("{}", l); }
}
