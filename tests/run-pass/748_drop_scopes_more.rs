// More drop-scope rules: temporaries in if/while conditions (dropped before
// the body), match guards, let-else scrutinees, `?` early exits, for-loop
// iterator expressions (live for the whole loop), returns from the middle of
// expressions, struct literals, assignment statements and closures.
use std::cell::RefCell;

thread_local! { static EV: RefCell<Vec<String>> = RefCell::new(vec![]); }
fn ev(s: String) { EV.with(|e| e.borrow_mut().push(s)); }
fn dump(t: &str) { let v = EV.with(|e| std::mem::take(&mut *e.borrow_mut())); println!("{:<14}{}", t, v.join(" ")); }

struct G(&'static str);
impl Drop for G { fn drop(&mut self) { ev(format!("~{}", self.0)); } }
impl G {
    fn v(&self) -> i32 { ev(format!("v({})", self.0)); self.0.len() as i32 }
    fn items(&self) -> Vec<i32> { vec![1, 2] }
    fn res(&self, ok: bool) -> Result<i32, String> { if ok { Ok(1) } else { Err(format!("err from {}", self.0)) } }
}

struct Pair { a: i32, b: i32 }

fn with_q(ok: bool) -> Result<i32, String> {
    let _guard = G("q-guard");
    let x = G("q-temp").res(ok)? + G("q-temp2").v();
    ev(format!("x={}", x));
    Ok(x)
}

fn early(n: i32) -> i32 {
    let _g = G("early-local");
    let r = G("lhs").v() + if n > 0 { return G("ret").v() * 100 } else { n };
    r
}

fn main() {
    if G("if-cond").v() > 0 { ev("if-body".into()); } else { ev("else-body".into()); }
    dump("if cond:");

    if G("ifc2").v() < 0 { ev("then".into()); } else if G("elif").v() > 0 { ev("elif-body".into()); }
    dump("else-if:");

    let mut n = 0;
    while G("w").v() > n { ev(format!("body{}", n)); n += 2; }
    dump("while cond:");

    match 5 {
        x if G("guard1").v() > 10 => ev(format!("arm1 {}", x)),
        x if G("guard2").v() > 1 => ev(format!("arm2 {}", x)),
        _ => ev("arm3".into()),
    }
    dump("match guards:");

    let Some(k) = G("letelse").items().first().copied() else { panic!() };
    ev(format!("k={}", k));
    dump("let-else:");

    let r1 = with_q(true);
    ev(format!("{:?}", r1));
    dump("? ok:");
    let r2 = with_q(false);
    ev(format!("{:?}", r2));
    dump("? err:");

    for i in G("for-iter").items() {
        ev(format!("iter{}", i));
    }
    ev("after-for".into());
    dump("for expr:");

    let e1 = early(1);
    ev(format!("early={}", e1));
    dump("return mid:");
    let e2 = early(-1);
    ev(format!("early={}", e2));
    dump("no return:");

    let p = Pair { a: G("fa").v(), b: G("fb").v() };
    ev(format!("pair {} {}", p.a, p.b));
    dump("struct lit:");

    let mut slot = 0;
    slot = G("assign").v() + slot;
    ev(format!("slot={}", slot));
    slot += G("compound").v();
    ev(format!("slot={}", slot));
    dump("assignment:");

    let f = |g: G| g.v() * 2;
    let out = f(G("closure-arg"));
    ev(format!("out={}", out));
    dump("closure arg:");

    let b = G("and-l").v() > 0 && G("and-r").v() > 100 || G("or-r").v() > 0;
    ev(format!("b={}", b));
    dump("bool chain:");

    if let Some(x) = Some(G("iflet-some").v()) { ev(format!("got {}", x)); }
    dump("if-let some:");

    let arr = [10, 20, 30, 40, 50, 60];
    let el = arr[G("idx").v() as usize];
    ev(format!("el={}", el));
    dump("index temp:");

    let len = { G("blk1"); G("blk2").v() };
    ev(format!("len={}", len));
    dump("stmt temps:");
}
