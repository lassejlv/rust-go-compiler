use std::panic::catch_unwind;

struct D(&'static str);
impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}
impl D {
    fn get(&self, fail: bool) -> u32 {
        if fail {
            panic!("fail in {}", self.0);
        }
        1
    }
    fn name(&self) -> &str {
        self.0
    }
}

struct Pair(D, D);

fn mk(s: &'static str) -> D {
    D(s)
}

fn with_param(p: D, fail: bool) -> u32 {
    let _l = D("fn local");
    mk("fn tail").get(fail) + p.get(false)
}

fn scrutinee(fail: bool) -> u32 {
    let _l = D("fn local");
    match mk("scrut").get(false) {
        1 => {
            let _a = D("arm local");
            mk("arm tail").get(fail)
        }
        _ => 0,
    }
}

fn use_ref(s: &str) -> usize {
    println!("use_ref {}", s);
    s.len()
}

fn ref_tail() -> usize {
    let n = use_ref({
        let _l = D("block local");
        mk("borrowed tail").name()
    });
    println!("after call");
    n
}

fn fields(fail: bool) -> u32 {
    let _l = D("fields local");
    let p = Pair(D("p0"), D("p1"));
    {
        let _m = D("inner");
        Pair(mk("t0"), mk("t1")).0.get(fail) + p.1.get(false)
    }
}

fn loop_tail(fail: bool) -> u32 {
    let mut s = 0;
    for i in 0..2 {
        let _it = D("iter local");
        s += mk("loop body temp").get(fail && i == 1);
    }
    s
}

fn if_tail(fail: bool) -> u32 {
    let _l = D("if fn local");
    if mk("cond").get(false) == 1 {
        let _t = D("then local");
        mk("then tail").get(fail)
    } else {
        2
    }
}

fn question(o: Option<u32>) -> Option<u32> {
    let _l = D("q local");
    let r = {
        let _b = D("q block local");
        mk("q tail").get(false) + o?
    };
    Some(r)
}

fn main() {
    println!("-- with_param");
    with_param(D("param"), false);
    let _ = catch_unwind(|| with_param(D("param"), true));
    println!("-- scrutinee");
    scrutinee(false);
    let _ = catch_unwind(|| scrutinee(true));
    println!("-- ref_tail");
    ref_tail();
    println!("-- fields");
    fields(false);
    let _ = catch_unwind(|| fields(true));
    println!("-- loop");
    loop_tail(false);
    let _ = catch_unwind(|| loop_tail(true));
    println!("-- if");
    if_tail(false);
    let _ = catch_unwind(|| if_tail(true));
    println!("-- question");
    println!("{:?}", question(Some(1)));
    println!("{:?}", question(None));
    println!("-- closure with capture");
    let cap = D("captured");
    let c = move |fail: bool| {
        let _l = D("closure local");
        mk("closure tail").get(fail) + cap.get(false)
    };
    c(false);
    let _ = catch_unwind(move || c(true));
    println!("end");
}
