use std::cell::RefCell;
use std::rc::Rc;

struct Log(Rc<RefCell<Vec<String>>>, String);
impl Drop for Log {
    fn drop(&mut self) {
        self.0.borrow_mut().push(format!("drop {}", self.1));
    }
}

struct Pair { first: Log, second: Log }

fn main() {
    let log = Rc::new(RefCell::new(Vec::new()));
    let mk = |s: &str| Log(log.clone(), s.to_string());
    {
        let _a = mk("a");
        let _b = mk("b");
        let _p = Pair { first: mk("p1"), second: mk("p2") };
        let _t = (mk("t1"), mk("t2"));
        let _arr = [mk("arr0"), mk("arr1")];
    }
    log.borrow_mut().push("--".into());
    {
        let v = vec![mk("v0"), mk("v1"), mk("v2")];
        let mut it = v.into_iter();
        let first = it.next();
        drop(it);
        log.borrow_mut().push("after it".into());
        drop(first);
    }
    log.borrow_mut().push("--".into());
    {
        let mut x = mk("x1");
        x = mk("x2");
        log.borrow_mut().push("assigned".into());
        let _ = mk("underscore");
        let _keep = mk("keep");
        let _ = x;
    }
    log.borrow_mut().push("--".into());
    {
        let p = Box::into_raw(Box::new(mk("raw")));
        let o = Some(mk("opt"));
        if let Some(ref l) = o {
            log.borrow_mut().push(format!("peek {}", l.1));
        }
        unsafe { drop(Box::from_raw(p)); }
        let s = std::mem::ManuallyDrop::new(mk("leak"));
        let _ = &s;
    }
    log.borrow_mut().push("--".into());
    {
        let p = Pair { first: mk("m1"), second: mk("m2") };
        let Pair { second, .. } = p;
        log.borrow_mut().push("moved second".into());
        drop(second);
    }
    for l in log.borrow().iter() {
        println!("{}", l);
    }
}
