// Partial moves out of structs/tuples, conditional moves with drop flags,
// moves in loops and closures, and drop of remaining fields.
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Debug)]
struct Noisy {
    name: String,
    log: Rc<RefCell<Vec<String>>>,
}
impl Drop for Noisy {
    fn drop(&mut self) {
        self.log.borrow_mut().push(format!("drop {}", self.name));
    }
}

struct Bag {
    left: Noisy,
    right: Noisy,
    label: String,
}

fn mk(log: &Rc<RefCell<Vec<String>>>, name: &str) -> Noisy {
    Noisy { name: name.to_string(), log: log.clone() }
}

fn consume(n: Noisy) -> usize {
    n.name.len()
}

fn report(log: &Rc<RefCell<Vec<String>>>, title: &str) {
    let v: Vec<String> = log.borrow_mut().drain(..).collect();
    println!("{:<14}| {}", title, v.join(", "));
}

fn main() {
    let log = Rc::new(RefCell::new(Vec::new()));

    {
        let bag = Bag { left: mk(&log, "L"), right: mk(&log, "R"), label: "bag".into() };
        let l = bag.left;
        log.borrow_mut().push(format!("took {} from {}", l.name, bag.label));
        drop(l);
        log.borrow_mut().push("end".into());
    }
    report(&log, "partial");

    for cond in [true, false] {
        {
            let a = mk(&log, "cond-a");
            let b = mk(&log, "cond-b");
            if cond {
                let n = consume(a);
                log.borrow_mut().push(format!("consumed {}", n));
            }
            log.borrow_mut().push("scope end".into());
            let _ = &b;
        }
        report(&log, if cond { "cond true" } else { "cond false" });
    }

    {
        let t = (mk(&log, "t0"), mk(&log, "t1"), mk(&log, "t2"));
        let (x, _, _) = t;
        log.borrow_mut().push(format!("bound {}", x.name));
    }
    report(&log, "tuple pat");

    {
        let t = (mk(&log, "u0"), mk(&log, "u1"));
        let (_, ref y) = t;
        log.borrow_mut().push(format!("ref {}", y.name));
    }
    report(&log, "ref pat");

    {
        let mut slot = Some(mk(&log, "slot"));
        for i in 0..3 {
            if let Some(n) = slot.take() {
                log.borrow_mut().push(format!("iter {} got {}", i, n.name));
            } else {
                log.borrow_mut().push(format!("iter {} empty", i));
            }
        }
    }
    report(&log, "take loop");

    {
        let n = mk(&log, "closure");
        let f = move || n.name.clone();
        log.borrow_mut().push(format!("call {}", f()));
        log.borrow_mut().push(format!("call {}", f()));
        drop(f);
        log.borrow_mut().push("closure dropped".into());
    }
    report(&log, "move closure");

    {
        let v = vec![mk(&log, "v0"), mk(&log, "v1"), mk(&log, "v2")];
        let mut it = v.into_iter();
        let first = it.next().unwrap();
        log.borrow_mut().push(format!("first {}", first.name));
        drop(it);
        log.borrow_mut().push("iter dropped".into());
    }
    report(&log, "into_iter");

    {
        let mut b = Bag { left: mk(&log, "bl"), right: mk(&log, "br"), label: "x".into() };
        b.right = mk(&log, "br2");
        log.borrow_mut().push("reassigned".into());
        let Bag { left, .. } = b;
        log.borrow_mut().push(format!("destructured {}", left.name));
    }
    report(&log, "reassign");

    {
        struct Duo(Noisy, Noisy);
        let d = Duo(mk(&log, "duo0"), mk(&log, "duo1"));
        let Duo(first, _) = d;
        log.borrow_mut().push(format!("kept {}", first.name));
    }
    report(&log, "tuple struct _");

    {
        let base = Bag { left: mk(&log, "base-l"), right: mk(&log, "base-r"), label: "base".into() };
        let derived = Bag { left: mk(&log, "new-l"), ..base };
        log.borrow_mut().push(format!("derived {} {} {}", derived.left.name, derived.right.name, derived.label));
    }
    report(&log, "struct update");
}
