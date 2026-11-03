// drop order: locals reverse, struct fields declaration order, temporaries in expressions/match/if let, shadowing, _ vs _x
struct D(String);
impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}
fn d(s: &str) -> D {
    println!("make {}", s);
    D(s.to_string())
}
impl D {
    fn val(&self) -> usize {
        self.0.len()
    }
    fn check(&self) -> Option<&str> {
        Some(&self.0)
    }
}

struct Holder {
    first: D,
    second: D,
    third: (D, D),
}

enum E {
    One(D),
    Two(D, D),
}

fn ret_temp() -> usize {
    d("in-return").val() + d("in-return-2").val()
}

fn main() {
    println!("-- locals");
    {
        let _a = d("a");
        let _b = d("b");
        let _ = d("underscore");
        let _c = d("c");
        println!("end of block");
    }
    println!("-- struct fields");
    {
        let _h = Holder { third: (d("t0"), d("t1")), second: d("second"), first: d("first") };
        println!("built holder");
    }
    println!("-- temporaries in statement");
    let n = d("temp1").val() + d("temp2").val();
    println!("n = {}", n);
    println!("-- temporary in let binding extended");
    let r = &d("extended");
    println!("using {}", r.0);
    println!("-- match scrutinee temporary");
    match d("scrutinee").check() {
        Some(s) => println!("matched {}", s),
        None => println!("none"),
    }
    println!("after match");
    println!("-- if let temporary (2021: lives to end of if-let else)");
    if let Some(s) = d("iflet").check() {
        println!("in if-let {}", s);
    } else {
        println!("else");
    }
    println!("after if let");
    println!("-- shadowing");
    let x = d("shadow1");
    let x = D(format!("{}+2", x.0));
    println!("x is {}", x.0);
    println!("-- enum variants");
    {
        let _e1 = E::Two(d("e2a"), d("e2b"));
        let _e2 = E::One(d("e1"));
    }
    println!("-- vec and box");
    {
        let v = vec![d("v0"), d("v1"), d("v2")];
        let _b = Box::new(d("boxed"));
        let mut it = v.into_iter();
        let first = it.next();
        println!("took first {:?}", first.as_ref().map(|d| d.0.as_str()));
        drop(it);
        println!("iterator dropped");
    }
    println!("-- reassignment drops old value");
    let mut y = d("y-old");
    y = d("y-new");
    println!("reassigned to {}", y.0);
    println!("-- function return temp");
    println!("ret {}", ret_temp());
    println!("-- tuple partial move");
    {
        let t = (d("tA"), d("tB"));
        let (a, _) = t;
        println!("moved {}", a.0);
    }
    println!("-- array pattern partial move");
    {
        let arr = [d("a0"), d("a1"), d("a2")];
        let [first, _, last] = arr;
        println!("moved {} {}", first.0, last.0);
    }
    {
        let arr = [d("b0"), d("b1"), d("b2"), d("b3")];
        let [_, mid @ .., _] = arr;
        println!("mid has {}", mid.len());
    }
    println!("-- closure capture drop");
    {
        let cap = d("captured");
        let f = move || cap.val();
        println!("f() = {}", f());
    }
    println!("-- block expression temps");
    let len = {
        let tmp = d("blocktmp");
        tmp.val() * 2
    };
    println!("len {}", len);
    println!("-- end of main");
}
