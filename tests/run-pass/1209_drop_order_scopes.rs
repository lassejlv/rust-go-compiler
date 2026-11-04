use std::cell::RefCell;
use std::panic;

thread_local! {
    static LOG: RefCell<Vec<String>> = RefCell::new(Vec::new());
}

fn log(s: String) {
    LOG.with(|l| l.borrow_mut().push(s));
}

fn dump(title: &str) {
    LOG.with(|l| {
        println!("{}: {}", title, l.borrow().join(" "));
        l.borrow_mut().clear();
    });
}

struct D(&'static str);
impl Drop for D {
    fn drop(&mut self) {
        log(format!("drop({})", self.0));
    }
}

impl D {
    fn val(&self) -> i32 {
        self.0.len() as i32
    }
    fn opt(&self) -> Option<i32> {
        Some(self.val())
    }
}

struct Pair {
    a: D,
    b: D,
    c: (D, D),
}

enum E {
    One(D),
    Two(D, D),
}

fn make(name: &'static str) -> D {
    log(format!("make({})", name));
    D(name)
}

fn let_else(x: Option<i32>) -> i32 {
    let _guard = make("guard");
    let Some(v) = x.map(|v| v + make("tmp_in_init").val()) else {
        log("else".to_string());
        return -1;
    };
    log(format!("got {}", v));
    v
}

struct Bomb(&'static str);
impl Drop for Bomb {
    fn drop(&mut self) {
        log(format!("bomb drop {}", self.0));
    }
}

fn main() {
    {
        let _a = make("a");
        let _b = make("b");
        let _ = make("underscore");
        let _c = make("c");
    }
    dump("locals");

    {
        let p = Pair { b: make("b"), a: make("a"), c: (make("c0"), make("c1")) };
        let _ = &p;
    }
    dump("struct fields");

    {
        let p = Pair { a: make("a"), b: make("b"), c: (make("c0"), make("c1")) };
        let Pair { b, .. } = p;
        log("moved b".to_string());
        drop(b);
        log("after drop b".to_string());
    }
    dump("partial move");

    {
        match make("scrut").opt() {
            Some(n) => log(format!("arm {}", n)),
            None => {}
        }
        log("after match".to_string());
    }
    dump("match scrutinee temp");

    {
        if let Some(n) = make("iflet").opt() {
            log(format!("then {}", n));
        }
        log("after iflet".to_string());
        let x = make("stmt").val() + make("stmt2").val();
        log(format!("x={}", x));
    }
    dump("if let temp");

    {
        let v = vec![make("v0"), make("v1"), make("v2")];
        let e = E::Two(make("e0"), make("e1"));
        let o = E::One(make("o"));
        drop(v);
        let _ = (e, o);
    }
    dump("vec/enum");

    {
        let mut x = make("x1");
        log("reassign".to_string());
        x = make("x2");
        log(format!("now {}", x.0));
    }
    dump("reassign");

    println!("{}", let_else(Some(1)));
    dump("let else some");
    println!("{}", let_else(None));
    dump("let else none");

    {
        let t = (make("t0"), make("t1"));
        let (_, y) = t;
        log(format!("y={}", y.0));
    }
    dump("tuple partial");

    {
        let r = &make("ref-temp-extended");
        log(format!("using {}", r.0));
    }
    dump("temp lifetime extension");

    {
        let b = Box::new(make("boxed"));
        let s: Vec<Box<D>> = vec![b, Box::new(make("boxed2"))];
        let mut it = s.into_iter();
        let first = it.next();
        log("got first".to_string());
        drop(it);
        log("dropped iter".to_string());
        drop(first);
    }
    dump("into_iter");

    // panic + catch_unwind with drops during unwinding
    let prev = panic::take_hook();
    panic::set_hook(Box::new(|_| {}));
    let res = panic::catch_unwind(|| {
        let _b1 = Bomb("outer");
        let _d = make("in-closure");
        {
            let _b2 = Bomb("inner");
            if true {
                panic!("boom");
            }
        }
        log("unreachable".to_string());
    });
    log(format!("caught={}", res.is_err()));
    dump("unwind");

    let res = panic::catch_unwind(|| -> i32 {
        let v: Vec<i32> = Vec::new();
        v[3]
    });
    println!("index panic caught: {}", res.is_err());

    let r = panic::catch_unwind(|| {
        panic::panic_any(42u32);
    });
    match r {
        Err(e) => println!("payload u32: {:?}", e.downcast_ref::<u32>()),
        Ok(_) => {}
    }
    let r = panic::catch_unwind(|| {
        panic!("formatted {}", 7);
    });
    if let Err(e) = r {
        println!("payload: {:?} {:?}", e.downcast_ref::<String>(), e.downcast_ref::<&str>());
    }
    let r = panic::catch_unwind(|| {
        panic!("static str");
    });
    if let Err(e) = r {
        println!("payload: {:?}", e.downcast_ref::<&str>());
    }
    panic::set_hook(prev);

    // labeled break with value & while let pop
    let mut stack = vec![1, 2, 3, 4, 5, 6];
    let found = 'outer: loop {
        while let Some(top) = stack.pop() {
            for k in 0..top {
                if k * top == 12 {
                    break 'outer Some((k, top));
                }
            }
        }
        break None;
    };
    println!("{:?} {:?}", found, stack);
    let v = 'blk: {
        if stack.len() > 3 {
            break 'blk "long";
        }
        "short"
    };
    println!("{}", v);
    let mut count = 0;
    'a: for i in 0..5 {
        'b: for j in 0..5 {
            if j > i {
                continue 'a;
            }
            if i + j == 6 {
                break 'a;
            }
            if j == 2 {
                continue 'b;
            }
            count += 1;
        }
    }
    println!("count={}", count);
}
