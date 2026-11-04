// A log analyzer exercising newer std APIs: LazyLock/OnceLock config,
// chunk_by, extract_if, pop_if, is_sorted, array::from_fn, each_ref, iter::zip,
// split_first_chunk, ilog/isqrt/div_ceil, from_utf8_lossy, Rc::make_mut,
// trait upcasting, Borrow-based lookups, and Result/Option collection.
use std::borrow::Borrow;
use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt;
use std::rc::Rc;
use std::sync::{LazyLock, OnceLock};

static LEVELS: LazyLock<HashMap<&'static str, u8>> = LazyLock::new(|| [("TRACE", 0), ("DEBUG", 1), ("INFO", 2), ("WARN", 3), ("ERROR", 4)].into_iter().collect());
static CONFIG: OnceLock<Config> = OnceLock::new();

#[derive(Debug)]
struct Config {
    min_level: u8,
    slow_ms: u32,
}

#[derive(Debug, Clone, PartialEq)]
struct Entry {
    ts: u32, // seconds since start
    level: u8,
    service: Rc<str>,
    msg: String,
    latency_ms: Option<u32>,
}

#[derive(Debug, PartialEq)]
enum LineError {
    TooShort(usize),
    BadTimestamp(String),
    UnknownLevel(String),
}

impl fmt::Display for LineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LineError::TooShort(n) => write!(f, "line has only {} fields", n),
            LineError::BadTimestamp(t) => write!(f, "bad timestamp {:?}", t),
            LineError::UnknownLevel(l) => write!(f, "unknown level {:?}", l),
        }
    }
}

fn parse_ts(s: &str) -> Option<u32> {
    let [h, m, sec] = s.split(':').map(|p| p.parse::<u32>().ok()).collect::<Option<Vec<_>>>()?.try_into().ok()?;
    (m < 60 && sec < 60).then_some(h * 3600 + m * 60 + sec)
}

fn parse_line(line: &str, services: &mut HashSet<Rc<str>>) -> Result<Entry, LineError> {
    let fields: Vec<&str> = line.splitn(4, ' ').collect();
    let Some(([ts, level, svc], rest)) = fields.split_first_chunk::<3>() else {
        return Err(LineError::TooShort(fields.len()));
    };
    let ts = parse_ts(ts).ok_or_else(|| LineError::BadTimestamp(ts.to_string()))?;
    let level = *LEVELS.get(level).ok_or_else(|| LineError::UnknownLevel(level.to_string()))?;
    let svc = svc.trim_matches(|c| c == '[' || c == ']');
    let service = match services.get(svc) {
        Some(s) => Rc::clone(s),
        None => {
            let s: Rc<str> = Rc::from(svc);
            services.insert(s.clone());
            s
        }
    };
    let msg = rest.first().map_or("", |m| m.trim_ascii()).to_string();
    let latency_ms = msg.split_ascii_whitespace().find_map(|w| w.strip_prefix("latency=")).and_then(|v| v.trim_end_matches("ms").parse().ok());
    Ok(Entry { ts, level, service, msg, latency_ms })
}

trait Named {
    fn name(&self) -> String;
}
trait Detector: Named {
    fn feed(&mut self, e: &Entry);
    fn findings(&self) -> Vec<String>;
}

struct SlowRequests {
    threshold: u32,
    hits: Vec<(u32, Rc<str>, u32)>,
}
impl Named for SlowRequests {
    fn name(&self) -> String {
        format!("slow>{}ms", self.threshold)
    }
}
impl Detector for SlowRequests {
    fn feed(&mut self, e: &Entry) {
        if let Some(l) = e.latency_ms.filter(|&l| l > self.threshold) {
            self.hits.push((e.ts, e.service.clone(), l));
        }
    }
    fn findings(&self) -> Vec<String> {
        self.hits.iter().map(|(t, s, l)| format!("{}@{} {}ms", s, t, l)).collect()
    }
}

struct ErrorBurst {
    window: u32,
    min: usize,
    recent: Vec<u32>,
    bursts: Vec<(u32, usize)>,
}
impl Named for ErrorBurst {
    fn name(&self) -> String {
        "error-burst".into()
    }
}
impl Detector for ErrorBurst {
    fn feed(&mut self, e: &Entry) {
        if e.level < 4 {
            return;
        }
        self.recent.push(e.ts);
        let cutoff = e.ts.saturating_sub(self.window);
        while self.recent.pop_if(|_| false).is_some() {}
        let fresh: Vec<u32> = self.recent.extract_if(.., |t| *t < cutoff).collect();
        let _ = fresh;
        if self.recent.len() >= self.min {
            if self.bursts.last().map_or(true, |(t, _)| e.ts - t > self.window) {
                self.bursts.push((e.ts, self.recent.len()));
            } else if let Some(last) = self.bursts.last_mut() {
                last.1 = last.1.max(self.recent.len());
            }
        }
    }
    fn findings(&self) -> Vec<String> {
        self.bursts.iter().map(|(t, n)| format!("burst at {} ({} errors)", t, n)).collect()
    }
}

fn describe(n: &dyn Named) -> String {
    format!("<{}>", n.name())
}

fn count_by<K, Q: ?Sized>(map: &HashMap<K, usize>, key: &Q) -> usize
where
    K: Borrow<Q> + std::hash::Hash + Eq,
    Q: std::hash::Hash + Eq,
{
    map.get(key).copied().unwrap_or(0)
}

fn gen_log() -> String {
    let services = ["api", "db", "auth", "cache"];
    let levels = ["DEBUG", "INFO", "INFO", "INFO", "WARN", "ERROR"];
    let mut s = String::new();
    let mut x = 17u32;
    let mut t = 3600 * 9;
    for i in 0..120 {
        x = x.wrapping_mul(1103515245).wrapping_add(12345) & 0x7fff_ffff;
        t += x % 7;
        let lvl = if (40..48).contains(&i) { "ERROR" } else { levels[(x >> 8) as usize % levels.len()] };
        let svc = services[(x >> 4) as usize % services.len()];
        let lat = (x >> 12) % 900;
        s += &format!("{:02}:{:02}:{:02} {} [{}] request id={} latency={}ms\n", t / 3600, t / 60 % 60, t % 60, lvl, svc, i, lat);
    }
    s += "09:99:00 INFO [api] bad minute\n";
    s += "short line\n";
    s += "10:00:00 FATAL [db] what\n";
    s += "10:00:01 INFO [db]\n";
    s
}

fn main() {
    let cfg = CONFIG.get_or_init(|| Config { min_level: 1, slow_ms: 800 });
    println!("config {:?}, again same: {}", cfg, std::ptr::eq(cfg, CONFIG.get().unwrap()));
    let text = gen_log();
    let mut services = HashSet::new();
    let (ok, errs): (Vec<_>, Vec<_>) = text.lines().map(|l| parse_line(l, &mut services)).partition(Result::is_ok);
    let entries: Vec<Entry> = ok.into_iter().map(Result::unwrap).filter(|e| e.level >= cfg.min_level).collect();
    let errs: Vec<String> = errs.into_iter().map(|e| e.unwrap_err().to_string()).collect();
    println!("{} entries, {} errors: {:?}", entries.len(), errs.len(), errs);
    println!("timestamps sorted: {}", entries.iter().map(|e| e.ts).is_sorted());
    let mut svc_names: Vec<&str> = services.iter().map(|s| s.as_ref()).collect();
    svc_names.sort_unstable();
    println!("services {:?}, api rc={}", svc_names, Rc::strong_count(services.get("api").unwrap()));

    // runs of equal levels
    let runs: Vec<(u8, usize)> = entries.chunk_by(|a, b| a.level == b.level).map(|c| (c[0].level, c.len())).filter(|r| r.1 > 2).collect();
    println!("level runs >2: {:?}", runs);

    // per-minute buckets
    let mut per_min: BTreeMap<u32, [u32; 5]> = BTreeMap::new();
    for e in &entries {
        per_min.entry(e.ts / 60).or_insert([0; 5])[e.level as usize] += 1;
    }
    let busiest = per_min.iter().max_by_key(|(m, c)| (c.iter().sum::<u32>(), std::cmp::Reverse(**m))).unwrap();
    println!("minutes {} busiest {:?}", per_min.len(), busiest);

    let mut dets: Vec<Box<dyn Detector>> = vec![
        Box::new(SlowRequests { threshold: cfg.slow_ms, hits: vec![] }),
        Box::new(ErrorBurst { window: 20, min: 4, recent: vec![], bursts: vec![] }),
    ];
    for e in &entries {
        for d in dets.iter_mut() {
            d.feed(e);
        }
    }
    for d in &dets {
        let named: &dyn Named = d.as_ref();
        let f = d.findings();
        println!("{} -> {} findings, first {:?}", describe(named), f.len(), f.iter().take(3).collect::<Vec<_>>());
    }

    // latency stats
    let lats: Vec<u32> = entries.iter().filter_map(|e| e.latency_ms).collect();
    let n = lats.len() as u32;
    let mean = lats.iter().sum::<u32>().div_ceil(n);
    let var = lats.iter().map(|&l| (l as i64 - mean as i64).pow(2) as u64).sum::<u64>() / n as u64;
    let mut sorted = lats.clone();
    sorted.sort_unstable();
    let pct = |p: u32| sorted[((sorted.len() as u32 * p).div_ceil(100) as usize).saturating_sub(1)];
    println!("lat n={} mean~{} sd~{} p50={} p90={} p99={} log2(max)={:?} log10={}", n, mean, var.isqrt(), pct(50), pct(90), pct(99), sorted.last().and_then(|m| m.checked_ilog2()), sorted.last().unwrap().ilog10());

    // arrays
    let squares: [u32; 6] = std::array::from_fn(|i| (i * i) as u32);
    let labels = ["a", "b", "c", "d", "e", "f"];
    let refs: [&u32; 6] = squares.each_ref();
    let zipped: Vec<String> = std::iter::zip(labels, refs).map(|(l, s)| format!("{}{}", l, s)).collect();
    println!("{:?} {:?} {}", squares.map(|x| x % 5), zipped, 17u32.next_multiple_of(5));

    // counting with Borrow lookups
    let mut by_svc: HashMap<String, usize> = HashMap::new();
    for e in &entries {
        *by_svc.entry(e.service.to_string()).or_default() += 1;
    }
    println!("api={} db={} nope={}", count_by(&by_svc, "api"), count_by(&by_svc, &"db".to_string()), count_by(&by_svc, "nope"));

    // Option/Result collection
    let maybe: Option<Vec<u32>> = ["1", "2", "3"].iter().map(|s| s.parse().ok()).collect();
    let bad: Result<Vec<u32>, _> = ["1", "x", "3"].iter().map(|s| s.parse::<u32>()).collect();
    let total: Option<u32> = [Some(1), Some(2), None].into_iter().sum();
    let prod: Result<u64, String> = [Ok(2u64), Ok(3), Ok(7)].into_iter().product();
    println!("{:?} {:?} {:?} {:?}", maybe, bad.map_err(|e| e.to_string()), total, prod);

    // bytes and utf8
    let raw = [b'h', b'i', 0xff, b'!', 0xe2, 0x82, 0xac];
    println!("{} {:?} {:?}", String::from_utf8_lossy(&raw), std::str::from_utf8(&raw[..2]), std::str::from_utf8(&raw).map_err(|e| e.valid_up_to()));
    let mut buf = [0u8; 4];
    let euro = '€'.encode_utf8(&mut buf);
    println!("{} {} {:?} {:#06x} {:#010b} {:+.2e}", euro.len(), '€' as u32, u32::from_be_bytes([0, 0, 0x20, 0xac]).to_le_bytes(), 255, 5, 12345.678);

    // Rc::make_mut copy-on-write and Cell/RefCell helpers
    let mut shared = Rc::new(vec![1, 2, 3]);
    let other = Rc::clone(&shared);
    Rc::make_mut(&mut shared).push(4);
    println!("cow: {:?} {:?}", shared, other);
    println!("unique-now={}", Rc::get_mut(&mut shared).is_some());
    let c = Cell::new(5);
    let old = c.take();
    let r = RefCell::new(vec![1]);
    let prev = r.replace_with(|v| v.iter().map(|x| x * 10).collect());
    println!("{} {} {:?} {:?}", old, c.get(), prev, r.borrow());
    let opt: Option<u32> = Some(4);
    println!("{} {} {:?}", opt.is_some_and(|x| x > 3), opt.is_none_or(|x| x > 9), opt.zip(Some('z')));
}
