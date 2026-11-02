// Closures capturing by move inside loops, capturing loop variables, capturing clones,
// disjoint field capture (2021), and closures that capture other closures.
#[derive(Debug, Clone)]
struct Config {
    name: String,
    level: u32,
    tags: Vec<String>,
}

fn run_all(fs: &[Box<dyn Fn() -> String>]) -> Vec<String> {
    fs.iter().map(|f| f()).collect()
}

fn main() {
    let mut handlers: Vec<Box<dyn Fn() -> String>> = Vec::new();
    for i in 0..3 {
        let label = format!("h{}", i);
        handlers.push(Box::new(move || format!("{}:{}", label, i * i)));
    }
    println!("{:?}", run_all(&handlers));

    let base = String::from("base");
    let mut clones: Vec<Box<dyn Fn(&str) -> String>> = Vec::new();
    for suffix in ["a", "b", "c"] {
        let b = base.clone();
        clones.push(Box::new(move |extra| format!("{}-{}-{}", b, suffix, extra)));
    }
    println!("base still usable: {}", base);
    for f in &clones {
        println!("{}", f("x"));
    }

    // disjoint capture: closure borrows cfg.tags mutably while cfg.name is read separately
    let mut cfg = Config { name: "svc".into(), level: 2, tags: vec![] };
    {
        let mut add_tag = |t: &str| cfg.tags.push(t.to_uppercase());
        add_tag("fast");
        add_tag("safe");
    }
    let name_ref = &cfg.name;
    let mut bump = || cfg.level += 10;
    bump();
    println!("{} {} {:?}", name_ref, cfg.level, cfg.tags);

    // move only one field (2021 precise capture), other field still usable
    let cfg2 = Config { name: "two".into(), level: 1, tags: vec!["t".into()] };
    let take_tags = move || cfg2.tags.len();
    println!("name still here: {}", cfg2.name);
    println!("tags len {}", take_tags());

    // closures capturing closures
    let mult = 3;
    let triple = move |x: i32| x * mult;
    let twice = move |x: i32| triple(triple(x));
    let thrice = move |x: i32| twice(x) + triple(x);
    println!("{} {}", twice(2), thrice(2));

    // building closures in while loop with mutable state snapshot
    let mut snapshots: Vec<Box<dyn Fn() -> Vec<u32>>> = Vec::new();
    let mut state: Vec<u32> = Vec::new();
    let mut n = 1;
    while n < 20 {
        state.push(n);
        let snap = state.clone();
        snapshots.push(Box::new(move || snap.iter().map(|v| v * 2).collect()));
        n *= 3;
    }
    for (i, s) in snapshots.iter().enumerate() {
        println!("snap {} -> {:?}", i, s());
    }

    // FnMut closures in a vec, each with own counter
    let mut counters: Vec<Box<dyn FnMut() -> u32>> = (1..=3u32)
        .map(|step| {
            let mut c = 0u32;
            Box::new(move || {
                c += step;
                c
            }) as Box<dyn FnMut() -> u32>
        })
        .collect();
    for _ in 0..3 {
        let vals: Vec<u32> = counters.iter_mut().map(|f| f()).collect();
        println!("{:?}", vals);
    }

    // move closure into thread-like consumer and return value out
    let data = vec![5, 1, 4];
    let sorter = move || {
        let mut d = data;
        d.sort();
        d
    };
    println!("sorted {:?}", sorter());

    let words = vec!["alpha", "beta", "gamma"];
    let lens: Vec<Box<dyn Fn() -> usize>> = words.iter().map(|w| {
        let w = *w;
        Box::new(move || w.len()) as Box<dyn Fn() -> usize>
    }).collect();
    println!("{:?}", lens.iter().map(|f| f()).collect::<Vec<_>>());
}
