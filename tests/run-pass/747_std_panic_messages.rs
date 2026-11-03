// Panic messages and caller locations of many std operations, captured by a
// panic hook that prints to stdout and by catch_unwind.
use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap, VecDeque};
use std::panic::{self, AssertUnwindSafe};

fn msg(p: &(dyn std::any::Any + Send)) -> String {
    p.downcast_ref::<&str>().map(|s| s.to_string()).or_else(|| p.downcast_ref::<String>().cloned()).unwrap_or_else(|| "?".into())
}

fn main() {
    panic::set_hook(Box::new(|info| {
        let l = info.location().unwrap();
        if l.file().contains("747_std_panic_messages") {
            println!("    at {}:{}", l.line(), l.column());
        } else {
            println!("    at <std>");
        }
    }));
    use std::hint::black_box as bb;
    let v = vec![1, 2, 3];
    let s = String::from("héllo");
    let cell = RefCell::new(5);
    let hm: HashMap<&str, i32> = HashMap::new();
    let bm: BTreeMap<i32, i32> = BTreeMap::new();
    let dq: VecDeque<i32> = VecDeque::from(vec![1]);
    let cases: Vec<(&str, Box<dyn Fn()>)> = vec![
        ("index", Box::new(|| { let _ = v[bb(5)]; })),
        ("range end", Box::new(|| { let _ = &v[1..bb(10)]; })),
        ("range start>end", Box::new(|| { let _ = &v[bb(2)..1]; })),
        ("range start", Box::new(|| { let _ = &v[bb(4)..]; })),
        ("str boundary", Box::new(|| { let _ = &s[..bb(2)]; })),
        ("str range", Box::new(|| { let _ = &s[bb(3)..20]; })),
        ("unwrap none", Box::new(|| { bb(None::<i32>).unwrap(); })),
        ("unwrap err", Box::new(|| { "x".parse::<i32>().unwrap(); })),
        ("expect err", Box::new(|| { "x".parse::<u8>().expect("wanted a number"); })),
        ("unwrap_err ok", Box::new(|| { Ok::<i32, String>(3).unwrap_err(); })),
        ("expect none", Box::new(|| { bb(None::<u8>).expect("value required"); })),
        ("sub overflow", Box::new(|| { let _ = bb(0u32) - 1; })),
        ("mul overflow", Box::new(|| { let _ = bb(i64::MAX) * 2; })),
        ("div zero", Box::new(|| { let _ = 1 / bb(0); })),
        ("rem zero", Box::new(|| { let _ = 1 % bb(0); })),
        ("div overflow", Box::new(|| { let _ = bb(i32::MIN) / -1; })),
        ("pow overflow", Box::new(|| { let _ = bb(10u8).pow(3); })),
        ("abs overflow", Box::new(|| { let _ = bb(i8::MIN).abs(); })),
        ("borrow_mut twice", Box::new(|| { let _a = cell.borrow(); let _b = cell.borrow_mut(); })),
        ("borrow while mut", Box::new(|| { let _a = cell.borrow_mut(); let _b = cell.borrow(); })),
        ("hashmap index", Box::new(|| { let _ = hm["missing"]; })),
        ("btreemap index", Box::new(|| { let _ = bm[&7]; })),
        ("vecdeque index", Box::new(|| { let _ = dq[bb(3)]; })),
        ("vec remove", Box::new(|| { let mut w = v.clone(); w.remove(bb(5)); })),
        ("vec insert", Box::new(|| { let mut w = v.clone(); w.insert(bb(7), 0); })),
        ("swap_remove", Box::new(|| { let mut w = v.clone(); w.swap_remove(bb(3)); })),
        ("split_at", Box::new(|| { let _ = v.split_at(bb(4)); })),
        ("copy_from_slice", Box::new(|| { let mut a = [0; 3]; a.copy_from_slice(&v[..bb(2)]); })),
        ("drain range", Box::new(|| { let mut w = v.clone(); w.drain(bb(2)..5); })),
        ("chunks zero", Box::new(|| { let _ = v.chunks(bb(0)).count(); })),
        ("step_by zero", Box::new(|| { let _ = (0..5).step_by(bb(0)); })),
        ("from_digit radix", Box::new(|| { let _ = char::from_digit(1, bb(40)); })),
        ("string remove", Box::new(|| { let mut t = s.clone(); t.remove(bb(2)); })),
        ("string insert", Box::new(|| { let mut t = s.clone(); t.insert(bb(2), 'x'); })),
        ("truncate boundary", Box::new(|| { let mut t = s.clone(); t.truncate(bb(2)); })),
        ("unreachable", Box::new(|| { if bb(true) { unreachable!("state {}", 3) } })),
        ("todo", Box::new(|| { if bb(true) { todo!() } })),
        ("unimplemented", Box::new(|| { if bb(true) { unimplemented!("feature {}", "x") } })),
        ("assert", Box::new(|| { assert!(bb(1) > 2); })),
        ("assert msg", Box::new(|| { assert!(bb(1) > 2, "custom {} msg", 42); })),
        ("assert_ne", Box::new(|| { assert_ne!(bb(4), 4); })),
        ("assert_eq strs", Box::new(|| { assert_eq!(bb("a"), "b", "texts differ"); })),
        ("debug_assert", Box::new(|| { debug_assert!(bb(false)); })),
        ("array index", Box::new(|| { let a = [1u8; 4]; let _ = a[bb(4)]; })),
        ("slice to_vec idx", Box::new(|| { let a: &[u8] = &[]; let _ = a[bb(0)]; })),
        ("shr overflow", Box::new(|| { let _ = 1u64 >> bb(64); })),
        ("cast ok", Box::new(|| { let _ = bb(300) as u8; })),
    ];
    for (name, f) in &cases {
        println!("{}:", name);
        match panic::catch_unwind(AssertUnwindSafe(|| f())) {
            Ok(()) => println!("    no panic"),
            Err(p) => println!("    msg: {}", msg(&*p)),
        }
    }
}
