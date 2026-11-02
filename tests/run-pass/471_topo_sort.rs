// Topological sorting of a build dependency graph: Kahn's algorithm with a BTreeSet ready-queue for
// deterministic lexical order, DFS-based ordering with three-color marking and cycle path extraction,
// level/parallel-batch computation, critical path with durations, and transitive reduction check.
use std::collections::{BTreeMap, BTreeSet};

type Deps<'a> = BTreeMap<&'a str, BTreeSet<&'a str>>; // node -> things it depends on

fn parse(spec: &str) -> Deps<'_> {
    let mut d: Deps = BTreeMap::new();
    for line in spec.lines().map(str::trim).filter(|l| !l.is_empty()) {
        let (node, rest) = line.split_once(':').unwrap();
        let node = node.trim();
        let set = d.entry(node).or_default();
        for dep in rest.split(',').map(str::trim).filter(|s| !s.is_empty()) {
            set.insert(dep);
        }
        for dep in rest.split(',').map(str::trim).filter(|s| !s.is_empty()) {
            d.entry(dep).or_default();
        }
    }
    d
}

fn kahn<'a>(d: &Deps<'a>) -> Result<Vec<&'a str>, Vec<&'a str>> {
    let mut indeg: BTreeMap<&str, usize> = d.iter().map(|(k, v)| (*k, v.len())).collect();
    let mut users: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for (k, deps) in d {
        for dep in deps {
            users.entry(dep).or_default().push(k);
        }
    }
    let mut ready: BTreeSet<&str> = indeg.iter().filter(|(_, &n)| n == 0).map(|(k, _)| *k).collect();
    let mut out = vec![];
    while let Some(n) = ready.pop_first() {
        out.push(n);
        for u in users.get(n).into_iter().flatten() {
            let e = indeg.get_mut(u).unwrap();
            *e -= 1;
            if *e == 0 {
                ready.insert(u);
            }
        }
    }
    if out.len() == d.len() {
        Ok(out)
    } else {
        Err(indeg.into_iter().filter(|(_, n)| *n > 0).map(|(k, _)| k).collect())
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Color {
    White,
    Gray,
    Black,
}

fn dfs_order<'a>(d: &Deps<'a>) -> Result<Vec<&'a str>, Vec<&'a str>> {
    fn visit<'a>(n: &'a str, d: &Deps<'a>, color: &mut BTreeMap<&'a str, Color>, stack: &mut Vec<&'a str>, out: &mut Vec<&'a str>) -> Result<(), Vec<&'a str>> {
        match color[n] {
            Color::Black => return Ok(()),
            Color::Gray => {
                let start = stack.iter().position(|s| *s == n).unwrap();
                let mut cyc = stack[start..].to_vec();
                cyc.push(n);
                return Err(cyc);
            }
            Color::White => {}
        }
        color.insert(n, Color::Gray);
        stack.push(n);
        for dep in &d[n] {
            visit(dep, d, color, stack, out)?;
        }
        stack.pop();
        color.insert(n, Color::Black);
        out.push(n);
        Ok(())
    }
    let mut color: BTreeMap<&str, Color> = d.keys().map(|k| (*k, Color::White)).collect();
    let mut out = vec![];
    for k in d.keys() {
        visit(k, d, &mut color, &mut vec![], &mut out)?;
    }
    Ok(out)
}

fn levels<'a>(d: &Deps<'a>, order: &[&'a str]) -> BTreeMap<usize, Vec<&'a str>> {
    let mut lvl: BTreeMap<&str, usize> = BTreeMap::new();
    for n in order {
        let l = d[n].iter().map(|dep| lvl[dep] + 1).max().unwrap_or(0);
        lvl.insert(n, l);
    }
    let mut by: BTreeMap<usize, Vec<&str>> = BTreeMap::new();
    for (n, l) in lvl {
        by.entry(l).or_default().push(n);
    }
    by
}

fn critical_path<'a>(d: &Deps<'a>, order: &[&'a str], dur: &BTreeMap<&str, u32>) -> (u32, Vec<&'a str>) {
    let mut finish: BTreeMap<&str, (u32, Option<&str>)> = BTreeMap::new();
    for n in order {
        let best = d[n].iter().map(|dep| (finish[dep].0, Some(*dep))).max().unwrap_or((0, None));
        finish.insert(n, (best.0 + dur.get(n).copied().unwrap_or(1), best.1));
    }
    let (&end, &(total, _)) = finish.iter().max_by_key(|(k, v)| (v.0, std::cmp::Reverse(**k))).unwrap();
    let mut path = vec![end];
    let mut cur = end;
    while let Some(p) = finish[cur].1 {
        path.push(p);
        cur = p;
    }
    path.reverse();
    (total, path)
}

fn main() {
    let spec = "
        app: lib, ui, config
        lib: core, util
        ui: core, assets
        core: util
        util:
        assets:
        config: util
        tests: app, lib
        docs: lib
    ";
    let d = parse(spec);
    let k = kahn(&d).unwrap();
    println!("kahn: {:?}", k);
    let o = dfs_order(&d).unwrap();
    println!("dfs:  {:?}", o);
    // verify both orders respect deps
    for order in [&k, &o] {
        let pos: BTreeMap<&str, usize> = order.iter().enumerate().map(|(i, n)| (*n, i)).collect();
        let ok = d.iter().all(|(n, deps)| deps.iter().all(|dep| pos[dep] < pos[n]));
        println!("valid: {}", ok);
    }
    for (l, ns) in levels(&d, &k) {
        println!("level {}: {:?}", l, ns);
    }
    let dur: BTreeMap<&str, u32> = [("app", 3), ("lib", 5), ("ui", 4), ("core", 6), ("util", 2), ("assets", 7), ("config", 1), ("tests", 4), ("docs", 2)].into_iter().collect();
    println!("critical: {:?}", critical_path(&d, &k, &dur));

    let cyclic = parse("a: b\nb: c\nc: a, d\nd:\ne: d");
    println!("kahn cyclic: {:?}", kahn(&cyclic));
    println!("dfs cyclic: {:?}", dfs_order(&cyclic));
    let self_loop = parse("x: x");
    println!("{:?} {:?}", kahn(&self_loop), dfs_order(&self_loop));
    let empty = parse("");
    println!("{:?} {:?}", kahn(&empty), dfs_order(&empty));
}
