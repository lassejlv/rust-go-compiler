// Temporary lifetime extension rules for `let`: extended through &, tuple,
// array, struct literals, casts and block tails; NOT extended through method
// receivers, function arguments, index operands, or field projections of
// non-borrowed temporaries. Drop events show when each temporary dies.
use std::cell::RefCell;

thread_local! { static EV: RefCell<Vec<String>> = RefCell::new(vec![]); }
fn ev(s: String) { EV.with(|e| e.borrow_mut().push(s)); }
fn dump(t: &str) { let v = EV.with(|e| std::mem::take(&mut *e.borrow_mut())); println!("{:<16}{}", t, v.join(" ")); }

struct T { name: &'static str, n: i32 }
impl Drop for T { fn drop(&mut self) { ev(format!("~{}", self.name)); } }
fn t(name: &'static str) -> T { ev(format!("+{}", name)); T { name, n: name.len() as i32 } }
impl T { fn get(&self) -> i32 { self.n } fn me(&self) -> &T { self } }

struct Holder<'a> { r: &'a T, k: i32 }
fn id<X>(x: X) -> X { x }

fn main() {
    {
        let r = &t("borrow");
        ev(format!("use {}", r.n));
    }
    dump("&temp:");
    {
        let (a, b) = (&t("tup0"), t("tup1").get());
        ev(format!("use {} {}", a.n, b));
    }
    dump("tuple:");
    {
        let arr = [&t("arr0"), &t("arr1")];
        ev(format!("use {}", arr.len()));
    }
    dump("array:");
    {
        let h = Holder { r: &t("field"), k: t("plain").get() };
        ev(format!("use {} {}", h.r.n, h.k));
    }
    dump("struct lit:");
    {
        let r = { ev("in block".into()); &t("blocktail") };
        ev(format!("use {}", r.n));
    }
    dump("block tail:");
    {
        let r = &t("proj").n;
        ev(format!("use {}", r));
    }
    dump("&temp.field:");
    {
        let v = t("method").me().get();
        ev(format!("use {}", v));
    }
    dump("method recv:");
    {
        let v = id(&t("fnarg")).get();
        ev(format!("use {}", v));
    }
    dump("fn arg:");
    {
        let n = t("byval").n;
        ev(format!("use {}", n));
    }
    dump("field of temp:");
    {
        let x = &(t("paren"));
        let y: &dyn std::any::Any = &t("cast").n;
        ev(format!("use {} {}", x.n, y.is::<i32>()));
    }
    dump("paren/cast:");
    {
        let s = &*Box::new(t("deref"));
        ev(format!("use {}", s.n));
    }
    dump("&*Box::new:");
    {
        let r = if true { &t("if-arm") } else { &t("else-arm") };
        ev(format!("use {}", r.n));
    }
    dump("if arms:");
    {
        let v = vec![t("v0"), t("v1")];
        let first = &v[0];
        ev(format!("use {}", first.n));
    }
    dump("vec owned:");
    {
        let m = match t("scrut").get() { 6 => t("arm6").n, _ => 0 };
        ev(format!("use {}", m));
    }
    dump("match value:");
    {
        let o: Option<&T> = Some(&t("some-arg"));
        let n = o.map(|x| x.n);
        ev(format!("use {:?}", n));
    }
    dump("Some(&temp):");
}
