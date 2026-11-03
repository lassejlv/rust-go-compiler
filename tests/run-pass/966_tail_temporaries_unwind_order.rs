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
}

fn mk(s: &'static str) -> D {
    D(s)
}

fn f(fail: bool) -> u32 {
    let _a = D("local a");
    let _b = D("local b");
    mk("tail temp").get(fail)
}

fn g(fail: bool) -> u32 {
    let x = {
        let _c = D("inner local");
        mk("inner tail temp").get(fail) + mk("second temp").get(false)
    };
    x
}

fn h(fail: bool) -> u32 {
    let _l = D("h local");
    match mk("scrutinee").get(false) {
        1 => mk("arm temp").get(fail),
        _ => 0,
    }
}

fn main() {
    println!("-- f ok");
    f(false);
    println!("-- f panic");
    let _ = catch_unwind(|| f(true));
    println!("-- g ok");
    g(false);
    println!("-- g panic");
    let _ = catch_unwind(|| g(true));
    println!("-- h ok");
    h(false);
    println!("-- h panic");
    let _ = catch_unwind(|| h(true));
    println!("-- closure");
    let _ = catch_unwind(|| {
        let _q = D("closure local");
        mk("closure tail").get(true)
    });
    println!("-- unsafe/nested block");
    let _ = catch_unwind(|| {
        let _q = D("outer local");
        {
            let _r = D("nested local");
            mk("nested tail").get(true)
        }
    });
}
