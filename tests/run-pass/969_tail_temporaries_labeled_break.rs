use std::panic::catch_unwind;

struct D(String);
impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}
impl D {
    fn v(&self, fail: bool) -> u32 {
        if fail {
            panic!("fail {}", self.0);
        }
        self.0.len() as u32
    }
    fn s(&self) -> &str {
        &self.0
    }
}
fn d(s: &str) -> D {
    D(s.to_string())
}

fn labeled(c: bool) -> u32 {
    let _x = d("lab fn local");
    'a: {
        let _y = d("lab local");
        if c {
            break 'a d("break temp").v(false);
        }
        d("lab tail").v(false)
    }
}

fn looped(n: u32) -> u32 {
    let mut i = 0;
    loop {
        let _l = d("loop local");
        i += 1;
        if i == n {
            break {
                let _b = d("break block local");
                d("break block tail").v(false)
            };
        }
    }
}

fn opt(o: Option<u32>) -> Option<u32> {
    let _x = d("opt local");
    Some({
        let _y = d("opt block local");
        d("opt tail").v(false) + o?
    })
}

fn res(o: Result<u32, String>) -> Result<u32, String> {
    let _x = d("res local");
    let r = {
        let _y = d("res block local");
        o.map(|v| v + d("map temp").v(false))?
    };
    Ok(r)
}

fn guard(n: u32, fail: bool) -> u32 {
    let _x = d("guard local");
    match n {
        k if d("guard temp").v(false) > k => {
            let _a = d("guard arm local");
            d("guard arm tail").v(fail)
        }
        _ => {
            let _b = d("other arm");
            7
        }
    }
}

fn strs(fail: bool) -> String {
    let _x = d("strs local");
    let s = {
        let _y = d("strs block local");
        format!("{}-{}", d("fmt temp 1").s(), {
            let _z = d("inner arg local");
            d("inner arg tail").v(fail)
        })
    };
    s
}

fn while_let(v: Vec<u32>) -> u32 {
    let mut it = v.into_iter();
    let mut t = 0;
    while let Some(x) = it.next() {
        let _l = d("wl local");
        t += {
            let _m = d("wl inner local");
            d("wl inner tail").v(false) + x
        };
    }
    t
}

fn main() {
    println!("{}", labeled(true));
    println!("{}", labeled(false));
    println!("{}", looped(2));
    println!("{:?}", opt(Some(1)));
    println!("{:?}", opt(None));
    println!("{:?}", res(Ok(1)));
    println!("{:?}", res(Err("e".into())));
    println!("{}", guard(1, false));
    println!("{}", guard(100, false));
    let _ = catch_unwind(|| guard(1, true));
    println!("{}", strs(false));
    let _ = catch_unwind(|| strs(true));
    println!("{}", while_let(vec![1, 2]));
    let r = catch_unwind(|| {
        let _c = d("closure local");
        let v = vec![d("vec elem")];
        {
            let _q = d("q");
            v[0].v(false) + d("t").v(true)
        }
    });
    println!("{}", r.is_err());
}
