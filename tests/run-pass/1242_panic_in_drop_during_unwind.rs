struct Loud(&'static str);
impl Drop for Loud { fn drop(&mut self) { println!("drop {}", self.0); if self.0 == "bomb" && std::thread::panicking() { println!("already panicking"); } } }
struct PanicOnDrop;
impl Drop for PanicOnDrop { fn drop(&mut self) { panic!("panic in drop"); } }
fn main() {
    std::panic::set_hook(Box::new(|info| { println!("hook: {}", info.payload().downcast_ref::<&str>().copied().unwrap_or("?")); }));
    let r = std::panic::catch_unwind(|| { let _a = Loud("a"); let _b = Loud("bomb"); panic!("first"); });
    println!("caught {}", r.is_err());
    let r = std::panic::catch_unwind(|| { let _p = PanicOnDrop; println!("body done"); });
    println!("caught drop panic {}", r.is_err());
    let r = std::panic::catch_unwind(|| { let v = vec![Loud("v1"), Loud("v2")]; let _x = Loud("x"); drop(v); std::panic::resume_unwind(Box::new(5u8)); });
    println!("{:?}", r.err().and_then(|e| e.downcast::<u8>().ok()));
    let h = std::thread::spawn(|| { let _t = Loud("thread"); panic!("in thread"); });
    println!("join err {}", h.join().is_err());
    let _z = Loud("z");
    let _p = PanicOnDrop;
    panic!("main panic");
}
