// Global registries with OnceLock/LazyLock: a type registry populated once, lazily-initialized
// statics accessed from many threads (initialized exactly once), and OnceCell-style memoization.
use std::any::TypeId;
use std::cell::OnceCell;
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{LazyLock, Mutex, OnceLock, RwLock};
use std::thread;

static INIT_CALLS: AtomicUsize = AtomicUsize::new(0);

static CONFIG: OnceLock<BTreeMap<&'static str, String>> = OnceLock::new();

fn config() -> &'static BTreeMap<&'static str, String> {
    CONFIG.get_or_init(|| {
        INIT_CALLS.fetch_add(1, Ordering::SeqCst);
        let mut m = BTreeMap::new();
        m.insert("theme", "dark".to_string());
        m.insert("font", "Mono 13".to_string());
        m.insert("tab_size", "4".to_string());
        m
    })
}

static PRIMES: LazyLock<Vec<u32>> = LazyLock::new(|| {
    INIT_CALLS.fetch_add(100, Ordering::SeqCst);
    let mut v = vec![];
    let mut n = 2;
    while v.len() < 50 {
        if v.iter().all(|p| n % p != 0) {
            v.push(n);
        }
        n += 1;
    }
    v
});

// A registry of "settings types" keyed by TypeId, filled at startup from several threads.
struct Registry {
    entries: RwLock<BTreeMap<&'static str, (TypeId, fn() -> String)>>,
}

static REGISTRY: LazyLock<Registry> = LazyLock::new(|| Registry { entries: RwLock::new(BTreeMap::new()) });

trait Setting: 'static {
    const KEY: &'static str;
    fn default_value() -> String;
}

struct FontSize;
struct Theme;
struct Vim;
impl Setting for FontSize {
    const KEY: &'static str = "font_size";
    fn default_value() -> String {
        "14".into()
    }
}
impl Setting for Theme {
    const KEY: &'static str = "theme";
    fn default_value() -> String {
        "One Dark".into()
    }
}
impl Setting for Vim {
    const KEY: &'static str = "vim_mode";
    fn default_value() -> String {
        "false".into()
    }
}

fn register<S: Setting>() -> bool {
    let mut w = REGISTRY.entries.write().unwrap();
    if w.contains_key(S::KEY) {
        return false;
    }
    w.insert(S::KEY, (TypeId::of::<S>(), S::default_value));
    true
}

struct Document {
    text: String,
    word_count: OnceCell<usize>,
    computed: std::cell::Cell<u32>,
}

impl Document {
    fn words(&self) -> usize {
        *self.word_count.get_or_init(|| {
            self.computed.set(self.computed.get() + 1);
            self.text.split_whitespace().count()
        })
    }
}

fn main() {
    let results: Vec<String> = thread::scope(|s| {
        let hs: Vec<_> = (0..8)
            .map(|i| {
                s.spawn(move || {
                    let c = config();
                    let p = PRIMES[i * 5];
                    format!("t{} theme={} prime={}", i, c["theme"], p)
                })
            })
            .collect();
        hs.into_iter().map(|h| h.join().unwrap()).collect()
    });
    for r in &results {
        println!("{}", r);
    }
    println!("init calls = {} (expect 101)", INIT_CALLS.load(Ordering::SeqCst));
    println!("config keys {:?}", config().keys().collect::<Vec<_>>());
    println!("primes len {} last {}", PRIMES.len(), PRIMES.last().unwrap());

    let outcomes = Mutex::new(Vec::new());
    thread::scope(|s| {
        for t in 0..6 {
            let outcomes = &outcomes;
            s.spawn(move || {
                let r = match t % 3 {
                    0 => ("font_size", register::<FontSize>()),
                    1 => ("theme", register::<Theme>()),
                    _ => ("vim_mode", register::<Vim>()),
                };
                outcomes.lock().unwrap().push(r);
            });
        }
    });
    let mut outcomes = outcomes.into_inner().unwrap();
    outcomes.sort();
    let firsts: BTreeMap<&str, usize> = outcomes.iter().fold(BTreeMap::new(), |mut m, (k, won)| {
        *m.entry(*k).or_insert(0) += *won as usize;
        m
    });
    println!("registration winners per key: {:?}", firsts);
    let reg = REGISTRY.entries.read().unwrap();
    for (k, (tid, f)) in reg.iter() {
        let kind = if *tid == TypeId::of::<FontSize>() {
            "FontSize"
        } else if *tid == TypeId::of::<Theme>() {
            "Theme"
        } else {
            "Vim"
        };
        println!("  {} -> {} default {:?}", k, kind, f());
    }

    let once: OnceLock<String> = OnceLock::new();
    println!("{:?}", once.get());
    println!("{:?}", once.set("first".into()));
    println!("{:?}", once.set("second".into()));
    println!("{:?} {:?}", once.get(), once.get_or_init(|| "unused".into()));

    let doc = Document { text: "a quick doc with six words".into(), word_count: OnceCell::new(), computed: std::cell::Cell::new(0) };
    println!("{} {} {} computed={}", doc.words(), doc.words(), doc.words(), doc.computed.get());
    let mut cell: OnceCell<Vec<u8>> = OnceCell::new();
    cell.get_or_init(|| vec![1, 2]);
    cell.get_mut().unwrap().push(3);
    println!("{:?} {:?}", cell.take(), cell.get());
}
