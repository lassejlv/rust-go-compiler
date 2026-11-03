// Drop order intricacies: struct fields, tuples, arrays, Vec elements,
// temporaries in match / if let scrutinees, shadowing, moved-out values,
// Option::take, early return, break with value, and closures capturing.
use std::cell::RefCell;

thread_local! { static LOG: RefCell<Vec<String>> = RefCell::new(Vec::new()); }

fn log(s: String) { LOG.with(|l| l.borrow_mut().push(s)); }
fn flush(title: &str) {
    let v = LOG.with(|l| std::mem::take(&mut *l.borrow_mut()));
    println!("{:<22} {}", title, v.join(" "));
}

struct D(&'static str);
impl Drop for D {
    fn drop(&mut self) { log(format!("~{}", self.0)); }
}
impl D {
    fn get(&self) -> Option<&'static str> { Some(self.0) }
    fn val(&self) -> u32 { self.0.len() as u32 }
}

struct Pair { a: D, b: D, c: (D, D) }

struct Outer { inner: Pair, last: D }
impl Drop for Outer {
    fn drop(&mut self) { log("~Outer(self first)".to_string()); }
}

fn make(n: &'static str) -> D { log(format!("+{}", n)); D(n) }

fn early(flag: bool) -> u32 {
    let _a = make("e1");
    let b = make("e2");
    if flag {
        return b.val();
    }
    let _c = make("e3");
    0
}

fn main() {
    {
        let _p = Pair { c: (make("c0"), make("c1")), b: make("b"), a: make("a") };
    }
    flush("struct fields:");
    {
        let _o = Outer { last: D("last"), inner: Pair { a: D("ia"), b: D("ib"), c: (D("x"), D("y")) } };
    }
    flush("outer with Drop:");
    {
        let _arr = [D("a0"), D("a1"), D("a2")];
        let _v = vec![D("v0"), D("v1")];
        let _t = (D("t0"), D("t1"));
    }
    flush("arr/vec/tuple:");
    {
        let x = D("first");
        let x = D("second");
        log(format!("using {}", x.0));
    }
    flush("shadowing:");
    {
        match make("scrut").get() {
            Some(n) => log(format!("arm {}", n)),
            None => {}
        }
        log("after-match".into());
    }
    flush("match temp:");
    {
        if let Some(n) = make("iflet").get() {
            log(format!("body {}", n));
        }
        log("after-iflet".into());
    }
    flush("if let temp:");
    {
        let n = make("stmt").val() + make("stmt2").val();
        log(format!("n={}", n));
    }
    flush("stmt temps:");
    {
        let a = D("moved");
        let b = D("kept");
        let v = vec![a];
        log("before vec drop".into());
        drop(v);
        log("after vec drop".into());
        let _ = b;
        log("after let _".into());
    }
    flush("moves:");
    {
        let mut opt = Some(D("opt"));
        let taken = opt.take();
        log(format!("taken is_some={}", taken.is_some()));
        opt = Some(D("opt2"));
        log("reassigned".into());
        let _ = opt;
    }
    flush("option take:");
    log(format!("ret={}", early(true)));
    flush("early return:");
    log(format!("ret={}", early(false)));
    flush("normal return:");
    {
        let r = loop {
            let _g = D("loopguard");
            break D("result");
        };
        log(format!("got {}", r.0));
    }
    flush("break value:");
    {
        let d = D("captured");
        let f = move || d.val();
        log(format!("call {}", f()));
        drop(f);
        log("closure dropped".into());
    }
    flush("closure:");
    {
        let mut v = vec![D("p0"), D("p1"), D("p2"), D("p3")];
        v.remove(1);
        log("removed".into());
        v.truncate(1);
        log("truncated".into());
        v.insert(0, D("p9"));
        let last = v.pop();
        log(format!("popped {:?}", last.as_ref().map(|d| d.0)));
    }
    flush("vec ops:");
    {
        let _a = D("assign-old");
        let mut s = D("s1");
        s = D("s2");
        log(format!("now {}", s.0));
    }
    flush("assignment:");
    {
        let b: Box<[D]> = vec![D("b0"), D("b1")].into_boxed_slice();
        let (x, y) = (D("x"), D("y"));
        let _keep = y;
        drop(x);
        log(format!("len {}", b.len()));
    }
    flush("box slice/destructure:");
}
