struct D(&'static str);
impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}
impl D {
    fn get(&self) -> u32 {
        1
    }
}

fn mk(s: &'static str) -> D {
    D(s)
}

fn nested() -> u32 {
    let _q = D("outer local");
    {
        let _r = D("nested local");
        mk("nested tail").get()
    }
}

fn in_let() -> u32 {
    let x = {
        let _q = D("outer local");
        {
            let _r = D("nested local");
            mk("nested tail").get()
        }
    };
    println!("after let");
    x
}

fn in_if(c: bool) -> u32 {
    let _q = D("fn local");
    if c {
        let _r = D("then local");
        mk("then tail").get()
    } else {
        0
    }
}

fn in_match(c: u32) -> u32 {
    let _q = D("fn local");
    match c {
        0 => {
            let _r = D("arm local");
            mk("arm tail").get()
        }
        _ => 0,
    }
}

fn in_stmt() {
    let _q = D("fn local");
    {
        let _r = D("stmt block local");
        mk("stmt block tail").get()
    };
    println!("after stmt");
}

fn main() {
    println!("-- nested");
    nested();
    println!("-- in_let");
    in_let();
    println!("-- in_if");
    in_if(true);
    println!("-- in_match");
    in_match(0);
    println!("-- in_stmt");
    in_stmt();
    println!("-- closure");
    let c = || {
        let _q = D("closure local");
        {
            let _r = D("closure nested local");
            mk("closure nested tail").get()
        }
    };
    c();
}
