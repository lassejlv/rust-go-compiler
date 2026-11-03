// Const items of Drop types (fresh value per use), statics with interior
// mutability (atomics, Mutex, OnceLock), struct-update syntax with Drop
// fields (which fields get moved vs dropped), and Default derivation.
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};

static DROPS: AtomicUsize = AtomicUsize::new(0);
static LOG: Mutex<Vec<String>> = Mutex::new(Vec::new());
static CONFIG: OnceLock<Vec<&'static str>> = OnceLock::new();

fn log(s: String) { LOG.lock().unwrap().push(s); }
fn flush(t: &str) {
    let v = std::mem::take(&mut *LOG.lock().unwrap());
    println!("{:<16}{}", t, v.join(" "));
}

#[derive(Debug, Clone, PartialEq)]
struct Noisy { name: &'static str }
impl Drop for Noisy {
    fn drop(&mut self) {
        DROPS.fetch_add(1, Ordering::SeqCst);
        log(format!("~{}", self.name));
    }
}

const PROTO: Noisy = Noisy { name: "proto" };
const PAIR: (u8, Noisy) = (1, Noisy { name: "pair" });

#[derive(Debug, Clone)]
struct Settings { title: Noisy, footer: Noisy, width: u32, tags: Vec<&'static str> }

impl Default for Settings {
    fn default() -> Self {
        log("default()".into());
        Settings { title: Noisy { name: "dtitle" }, footer: Noisy { name: "dfooter" }, width: 80, tags: vec!["d"] }
    }
}

#[derive(Debug, Default, PartialEq)]
struct Derived { a: u8, b: Option<String>, c: Vec<i32>, d: (bool, char), e: [u16; 3] }

fn config() -> &'static [&'static str] {
    CONFIG.get_or_init(|| {
        log("init config".into());
        vec!["alpha", "beta"]
    })
}

fn main() {
    {
        let _ = PROTO;
        log("after let _ = PROTO".into());
        let a = PROTO;
        let b = PROTO;
        log(format!("a==b {}", a == b));
        let n = PROTO.name.len();
        log(format!("n={}", n));
        let (k, p) = PAIR;
        log(format!("k={} p={}", k, p.name));
    }
    flush("const drops:");

    {
        let base = Settings { title: Noisy { name: "btitle" }, footer: Noisy { name: "bfooter" }, width: 100, tags: vec!["b"] };
        let s2 = Settings { title: Noisy { name: "newtitle" }, ..base };
        log(format!("s2 {} {} {}", s2.title.name, s2.footer.name, s2.width));
        log(format!("base.title still usable: {}", base.title.name));
    }
    flush("struct update:");

    {
        let s3 = Settings { width: 5, ..Default::default() };
        log(format!("s3 {} {}", s3.title.name, s3.width));
    }
    flush("update default:");

    {
        let s4 = Settings { title: Noisy { name: "t4" }, footer: Noisy { name: "f4" }, ..Settings::default() };
        log(format!("s4 width {} tags {:?}", s4.width, s4.tags));
    }
    flush("unused default:");

    {
        let orig = Settings::default();
        let mut c = orig.clone();
        c.title = Noisy { name: "replaced" };
        log(format!("c {}", c.title.name));
    }
    flush("clone+assign:");

    println!("config {:?} {:?}", config(), config().len());
    flush("oncelock:");
    println!("derived default {:?}", Derived::default());
    println!("derived eq {}", Derived::default() == Derived { a: 0, ..Default::default() });
    println!("total drops {}", DROPS.load(Ordering::SeqCst));
    static COUNTER: AtomicUsize = AtomicUsize::new(10);
    let prev: Vec<usize> = (0..3).map(|_| COUNTER.fetch_add(5, Ordering::Relaxed)).collect();
    println!("static counter prev {:?} now {} cas {:?} {:?}", prev, COUNTER.load(Ordering::Relaxed), COUNTER.compare_exchange(25, 1, Ordering::SeqCst, Ordering::SeqCst), COUNTER.compare_exchange(25, 2, Ordering::SeqCst, Ordering::SeqCst));
}
