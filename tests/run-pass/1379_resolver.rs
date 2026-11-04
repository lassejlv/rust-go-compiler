// A package manager core: semver parsing/ordering, version requirements
// (^, ~, >=, <, =, *, comma-joined), a registry index, a backtracking
// dependency resolver with conflict explanations, lockfile rendering,
// topological build ordering, and a parallel "build" on a thread pool
// (Arc<Mutex<..>> + Condvar) whose completion order is made deterministic.
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fmt;
use std::str::FromStr;
use std::sync::{Arc, Condvar, Mutex};
use std::thread;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct Version {
    major: u64,
    minor: u64,
    patch: u64,
    pre: Vec<String>,
}

impl Ord for Version {
    fn cmp(&self, o: &Self) -> Ordering {
        (self.major, self.minor, self.patch).cmp(&(o.major, o.minor, o.patch)).then_with(|| match (self.pre.is_empty(), o.pre.is_empty()) {
            (true, true) => Ordering::Equal,
            (true, false) => Ordering::Greater,
            (false, true) => Ordering::Less,
            (false, false) => {
                for (a, b) in self.pre.iter().zip(&o.pre) {
                    let c = match (a.parse::<u64>(), b.parse::<u64>()) {
                        (Ok(x), Ok(y)) => x.cmp(&y),
                        (Ok(_), Err(_)) => Ordering::Less,
                        (Err(_), Ok(_)) => Ordering::Greater,
                        (Err(_), Err(_)) => a.cmp(b),
                    };
                    if c != Ordering::Equal {
                        return c;
                    }
                }
                self.pre.len().cmp(&o.pre.len())
            }
        })
    }
}
impl PartialOrd for Version {
    fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
        Some(self.cmp(o))
    }
}

impl FromStr for Version {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, String> {
        let (core, pre) = match s.split_once('-') {
            Some((c, p)) => (c, p.split('.').map(String::from).collect()),
            None => (s, vec![]),
        };
        let nums: Vec<u64> = core.split('.').map(|p| p.parse::<u64>().map_err(|e| format!("bad version {:?}: {}", s, e))).collect::<Result<_, _>>()?;
        match nums[..] {
            [major, minor, patch] => Ok(Version { major, minor, patch, pre }),
            _ => Err(format!("bad version {:?}: expected 3 components", s)),
        }
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)?;
        if !self.pre.is_empty() {
            write!(f, "-{}", self.pre.join("."))?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
enum Cmp {
    Caret(Version),
    Tilde(Version),
    Ge(Version),
    Lt(Version),
    Exact(Version),
    Any,
}

#[derive(Debug, Clone, PartialEq)]
struct Req(Vec<Cmp>);

fn parse_partial(s: &str) -> Result<Version, String> {
    let mut parts: Vec<&str> = s.split('.').collect();
    while parts.len() < 3 {
        parts.push("0");
    }
    parts.join(".").parse()
}

impl FromStr for Req {
    type Err = String;
    fn from_str(s: &str) -> Result<Req, String> {
        s.split(',')
            .map(|p| {
                let p = p.trim();
                Ok(if p == "*" {
                    Cmp::Any
                } else if let Some(v) = p.strip_prefix(">=") {
                    Cmp::Ge(parse_partial(v.trim())?)
                } else if let Some(v) = p.strip_prefix('<') {
                    Cmp::Lt(parse_partial(v.trim())?)
                } else if let Some(v) = p.strip_prefix('=') {
                    Cmp::Exact(v.trim().parse()?)
                } else if let Some(v) = p.strip_prefix('~') {
                    Cmp::Tilde(parse_partial(v.trim())?)
                } else {
                    Cmp::Caret(parse_partial(p.strip_prefix('^').unwrap_or(p))?)
                })
            })
            .collect::<Result<Vec<_>, String>>()
            .map(Req)
    }
}

impl Cmp {
    fn matches(&self, v: &Version) -> bool {
        match self {
            Cmp::Any => v.pre.is_empty(),
            Cmp::Exact(x) => v == x,
            Cmp::Ge(x) => v >= x,
            Cmp::Lt(x) => v < x,
            Cmp::Tilde(x) => v >= x && v.major == x.major && v.minor == x.minor,
            Cmp::Caret(x) => {
                if v < x || (!v.pre.is_empty() && (v.major, v.minor, v.patch) != (x.major, x.minor, x.patch)) {
                    return false;
                }
                if x.major > 0 {
                    v.major == x.major
                } else if x.minor > 0 {
                    v.major == 0 && v.minor == x.minor
                } else {
                    v.major == 0 && v.minor == 0 && v.patch == x.patch
                }
            }
        }
    }
}

impl Req {
    fn matches(&self, v: &Version) -> bool {
        self.0.iter().all(|c| c.matches(v))
    }
}

impl fmt::Display for Req {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let parts: Vec<String> = self
            .0
            .iter()
            .map(|c| match c {
                Cmp::Any => "*".to_string(),
                Cmp::Caret(v) => format!("^{}", v),
                Cmp::Tilde(v) => format!("~{}", v),
                Cmp::Ge(v) => format!(">={}", v),
                Cmp::Lt(v) => format!("<{}", v),
                Cmp::Exact(v) => format!("={}", v),
            })
            .collect();
        f.write_str(&parts.join(", "))
    }
}

type Registry = BTreeMap<String, BTreeMap<Version, Vec<(String, Req)>>>;

fn registry() -> Registry {
    let raw: &[(&str, &str, &[(&str, &str)])] = &[
        ("app", "1.0.0", &[("web", "^2.1"), ("db", "^0.4"), ("log", "~1.2")]),
        ("web", "2.0.0", &[("http", "^1"), ("log", "^1")]),
        ("web", "2.1.0", &[("http", "^1.3"), ("log", "^1.2"), ("json", "^3")]),
        ("web", "2.2.0", &[("http", "^2"), ("log", "^1.3"), ("json", "^3")]),
        ("http", "1.2.0", &[("bytes", "^0.5")]),
        ("http", "1.3.1", &[("bytes", "^0.6")]),
        ("http", "1.4.0", &[("bytes", ">=0.6, <0.8")]),
        ("http", "2.0.0", &[("bytes", "^1")]),
        ("bytes", "0.5.9", &[]),
        ("bytes", "0.6.2", &[]),
        ("bytes", "0.7.0", &[]),
        ("bytes", "1.0.0", &[]),
        ("log", "1.2.0", &[]),
        ("log", "1.2.5", &[]),
        ("log", "1.3.0", &[]),
        ("log", "1.4.0-beta.1", &[]),
        ("json", "3.0.0", &[("bytes", "^0.6")]),
        ("json", "3.1.0", &[("bytes", "^0.7")]),
        ("db", "0.4.0", &[("bytes", "^0.6"), ("log", "^1.2")]),
        ("db", "0.4.3", &[("bytes", "^0.7"), ("log", "=1.3.0")]),
        ("db", "0.5.0", &[("bytes", "^1")]),
    ];
    let mut r = Registry::new();
    for (name, ver, deps) in raw {
        let deps = deps.iter().map(|(n, q)| (n.to_string(), q.parse::<Req>().unwrap())).collect();
        r.entry(name.to_string()).or_default().insert(ver.parse().unwrap(), deps);
    }
    r
}

#[derive(Default)]
struct Resolver<'r> {
    reg: Option<&'r Registry>,
    steps: usize,
    conflicts: Vec<String>,
}

impl<'r> Resolver<'r> {
    fn resolve(&mut self, chosen: &mut BTreeMap<String, Version>, pending: &mut Vec<(String, Req, String)>) -> bool {
        self.steps += 1;
        let Some((name, req, from)) = pending.pop() else { return true };
        if let Some(v) = chosen.get(&name) {
            if req.matches(v) {
                let ok = self.resolve(chosen, pending);
                if !ok {
                    pending.push((name, req, from));
                }
                return ok;
            }
            self.conflicts.push(format!("{} {} (chosen) does not satisfy {} required by {}", name, v, req, from));
            pending.push((name, req, from));
            return false;
        }
        let reg = self.reg.unwrap();
        let candidates: Vec<(&Version, &Vec<(String, Req)>)> = match reg.get(&name) {
            Some(vs) => vs.iter().rev().filter(|(v, _)| req.matches(v)).collect(),
            None => {
                self.conflicts.push(format!("no package named {:?} (required by {})", name, from));
                pending.push((name, req, from));
                return false;
            }
        };
        if candidates.is_empty() {
            self.conflicts.push(format!("no version of {} matches {} (required by {})", name, req, from));
        }
        for (v, deps) in candidates {
            chosen.insert(name.clone(), v.clone());
            let mark = pending.len();
            for (dn, dr) in deps.iter().rev() {
                pending.push((dn.clone(), dr.clone(), format!("{} {}", name, v)));
            }
            if self.resolve(chosen, pending) {
                return true;
            }
            pending.truncate(mark);
            chosen.remove(&name);
        }
        pending.push((name, req, from));
        false
    }
}

fn build_order(lock: &BTreeMap<String, Version>, reg: &Registry) -> Vec<Vec<String>> {
    let deps: BTreeMap<&String, Vec<&String>> = lock.iter().map(|(n, v)| (n, reg[n][v].iter().map(|(d, _)| lock.get_key_value(d).unwrap().0).collect())).collect();
    let mut level: HashMap<&String, usize> = HashMap::new();
    fn lvl<'a>(n: &'a String, deps: &BTreeMap<&'a String, Vec<&'a String>>, memo: &mut HashMap<&'a String, usize>) -> usize {
        if let Some(&l) = memo.get(n) {
            return l;
        }
        let l = deps[n].iter().map(|d| lvl(d, deps, memo) + 1).max().unwrap_or(0);
        memo.insert(n, l);
        l
    }
    for n in lock.keys() {
        lvl(n, &deps, &mut level);
    }
    let max = level.values().copied().max().unwrap_or(0);
    (0..=max).map(|l| lock.keys().filter(|n| level[n] == l).cloned().collect()).collect()
}

fn parallel_build(waves: &[Vec<String>], workers: usize) -> Vec<String> {
    let log = Arc::new(Mutex::new(Vec::new()));
    for (wi, wave) in waves.iter().enumerate() {
        let queue = Arc::new((Mutex::new(wave.clone()), Condvar::new()));
        let done = Arc::new(Mutex::new(BTreeSet::new()));
        let handles: Vec<_> = (0..workers)
            .map(|_| {
                let (queue, done) = (Arc::clone(&queue), Arc::clone(&done));
                thread::spawn(move || loop {
                    let job = {
                        let (lock, cv) = &*queue;
                        let mut q = lock.lock().unwrap();
                        let j = q.pop();
                        cv.notify_all();
                        j
                    };
                    match job {
                        Some(name) => {
                            let checksum = name.bytes().fold(7u32, |h, b| h.wrapping_mul(31).wrapping_add(b as u32));
                            done.lock().unwrap().insert(format!("{}#{:x}", name, checksum));
                        }
                        None => break,
                    }
                })
            })
            .collect();
        for h in handles {
            h.join().unwrap();
        }
        let finished: Vec<String> = done.lock().unwrap().iter().cloned().collect();
        log.lock().unwrap().push(format!("wave {}: {}", wi, finished.join(" ")));
    }
    Arc::try_unwrap(log).unwrap().into_inner().unwrap()
}

fn main() {
    let mut vs: Vec<Version> = ["1.0.0", "1.0.0-alpha", "1.0.0-alpha.1", "1.0.0-alpha.beta", "1.0.0-beta", "1.0.0-beta.2", "1.0.0-beta.11", "1.0.0-rc.1", "0.9.9", "1.0.1"].iter().map(|s| s.parse().unwrap()).collect();
    vs.sort();
    println!("{}", vs.iter().map(|v| v.to_string()).collect::<Vec<_>>().join(" < "));
    for bad in ["1.0", "1.x.0", "a.b.c-d"] {
        println!("{:?}", bad.parse::<Version>());
    }
    let reqs = ["^1.2.3", "^0.4", "^0.0.3", "~1.2", ">=1.0, <1.3", "=1.2.5", "*", "1.3"];
    let probe: Vec<Version> = ["1.2.0", "1.2.3", "1.2.9", "1.3.0", "2.0.0", "0.4.7", "0.5.0", "0.0.3", "0.0.4", "1.4.0-beta.1", "1.2.5"].iter().map(|s| s.parse().unwrap()).collect();
    for r in reqs {
        let req: Req = r.parse().unwrap();
        let hits: Vec<String> = probe.iter().filter(|v| req.matches(v)).map(|v| v.to_string()).collect();
        println!("{:<14} [{}] matches {:?}", r, req, hits);
    }

    let reg = registry();
    for root_req in ["^1", "=1.0.0"] {
        let mut res = Resolver { reg: Some(&reg), ..Default::default() };
        let mut chosen = BTreeMap::new();
        let mut pending = vec![("app".to_string(), root_req.parse::<Req>().unwrap(), "root".to_string())];
        let ok = res.resolve(&mut chosen, &mut pending);
        println!("resolve app {}: {} in {} steps, {} conflicts on the way", root_req, ok, res.steps, res.conflicts.len());
        for c in res.conflicts.iter().take(4) {
            println!("  conflict: {}", c);
        }
        if ok {
            println!("# lockfile");
            for (n, v) in &chosen {
                let deps = &reg[n][v];
                println!("{} = \"{}\"{}", n, v, if deps.is_empty() { String::new() } else { format!("  # deps: {}", deps.iter().map(|(d, r)| format!("{} {}", d, r)).collect::<Vec<_>>().join(", ")) });
            }
            let waves = build_order(&chosen, &reg);
            println!("build waves: {:?}", waves);
            for line in parallel_build(&waves, 3) {
                println!("{}", line);
            }
        }
    }
    // an unsatisfiable request explains itself
    let mut res = Resolver { reg: Some(&reg), ..Default::default() };
    let mut chosen = BTreeMap::new();
    let mut pending = vec![("db".to_string(), "^0.5".parse().unwrap(), "root".to_string()), ("json".to_string(), "^3".parse().unwrap(), "root".to_string()), ("ghost".to_string(), "*".parse().unwrap(), "root".to_string())];
    let ok = res.resolve(&mut chosen, &mut pending);
    println!("unsatisfiable: {} after {} steps", ok, res.steps);
    let mut uniq: Vec<&String> = res.conflicts.iter().collect();
    uniq.sort();
    uniq.dedup();
    for c in uniq {
        println!("  {}", c);
    }
}
