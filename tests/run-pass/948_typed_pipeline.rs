// Typed processing pipeline: stages with associated In/Out types, composed
// statically (Chain<A, B> where B::In == A::Out) and dynamically (type-erased).
use std::collections::BTreeMap;
use std::fmt::Debug;
use std::marker::PhantomData;

trait Stage {
    type In;
    type Out;
    fn name(&self) -> String;
    fn process(&mut self, input: Self::In) -> Option<Self::Out>;

    fn then<B>(self, next: B) -> Chain<Self, B>
    where
        Self: Sized,
        B: Stage<In = Self::Out>,
    {
        Chain { a: self, b: next }
    }
}

struct Chain<A, B> { a: A, b: B }
impl<A, B> Stage for Chain<A, B>
where
    A: Stage,
    B: Stage<In = A::Out>,
{
    type In = A::In;
    type Out = B::Out;
    fn name(&self) -> String { format!("{} -> {}", self.a.name(), self.b.name()) }
    fn process(&mut self, input: A::In) -> Option<B::Out> {
        let mid = self.a.process(input)?;
        self.b.process(mid)
    }
}

struct MapStage<I, O, F: FnMut(I) -> O> { label: &'static str, f: F, _p: PhantomData<fn(I) -> O> }
fn map<I, O, F: FnMut(I) -> O>(label: &'static str, f: F) -> MapStage<I, O, F> { MapStage { label, f, _p: PhantomData } }
impl<I, O, F: FnMut(I) -> O> Stage for MapStage<I, O, F> {
    type In = I;
    type Out = O;
    fn name(&self) -> String { self.label.to_string() }
    fn process(&mut self, input: I) -> Option<O> { Some((self.f)(input)) }
}

struct Filter<T, P: FnMut(&T) -> bool> { pred: P, dropped: usize, _p: PhantomData<T> }
fn filter<T, P: FnMut(&T) -> bool>(pred: P) -> Filter<T, P> { Filter { pred, dropped: 0, _p: PhantomData } }
impl<T, P: FnMut(&T) -> bool> Stage for Filter<T, P> {
    type In = T;
    type Out = T;
    fn name(&self) -> String { format!("filter(dropped={})", self.dropped) }
    fn process(&mut self, input: T) -> Option<T> {
        if (self.pred)(&input) { Some(input) } else { self.dropped += 1; None }
    }
}

struct Parse<T>(PhantomData<T>);
impl<T: std::str::FromStr> Stage for Parse<T> {
    type In = String;
    type Out = T;
    fn name(&self) -> String { format!("parse<{}>", std::any::type_name::<T>().rsplit("::").next().unwrap()) }
    fn process(&mut self, s: String) -> Option<T> { s.trim().parse().ok() }
}

#[derive(Default)]
struct Stats { n: u64, sum: i64, min: Option<i64>, max: Option<i64> }
impl Stage for Stats {
    type In = i64;
    type Out = (u64, i64);
    fn name(&self) -> String { "stats".into() }
    fn process(&mut self, x: i64) -> Option<(u64, i64)> {
        self.n += 1;
        self.sum += x;
        self.min = Some(self.min.map_or(x, |m| m.min(x)));
        self.max = Some(self.max.map_or(x, |m| m.max(x)));
        Some((self.n, self.sum))
    }
}

struct Counter<K: Ord> { counts: BTreeMap<K, usize> }
impl<K: Ord + Clone + Debug> Stage for Counter<K> {
    type In = K;
    type Out = usize;
    fn name(&self) -> String { format!("count({} keys)", self.counts.len()) }
    fn process(&mut self, k: K) -> Option<usize> {
        let c = self.counts.entry(k).or_insert(0);
        *c += 1;
        Some(*c)
    }
}

// Type erasure: any Stage<In = I, Out = O> can be boxed.
type DynStage<I, O> = Box<dyn Stage<In = I, Out = O>>;

fn run_all<S: Stage>(stage: &mut S, inputs: Vec<S::In>) -> Vec<S::Out> {
    inputs.into_iter().filter_map(|x| stage.process(x)).collect()
}

fn pick_normalizer(kind: &str) -> DynStage<String, String> {
    match kind {
        "upper" => Box::new(map("upper", |s: String| s.to_uppercase())),
        "trim" => Box::new(map("trim", |s: String| s.trim().to_string())),
        _ => Box::new(map("trim+lower", |s: String| s.trim().to_lowercase()).then(filter(|s: &String| !s.is_empty()))),
    }
}

impl<I, O> Stage for Box<dyn Stage<In = I, Out = O>> {
    type In = I;
    type Out = O;
    fn name(&self) -> String { format!("dyn[{}]", (**self).name()) }
    fn process(&mut self, i: I) -> Option<O> { (**self).process(i) }
}

fn main() {
    let raw: Vec<String> = vec![" 12", "x", "7 ", "-3", "", "40", "5"].into_iter().map(String::from).collect();

    let mut p = Parse::<i64>(PhantomData)
        .then(filter(|x: &i64| *x > 0))
        .then(map("square", |x: i64| x * x));
    println!("{}", p.name());
    println!("{:?}", run_all(&mut p, raw.clone()));
    println!("{}", p.name());

    let mut stats_pipe = Parse::<i64>(PhantomData).then(Stats::default());
    let outs = run_all(&mut stats_pipe, raw.clone());
    println!("{:?}", outs);
    let st = &stats_pipe.b;
    println!("n={} sum={} min={:?} max={:?}", st.n, st.sum, st.min, st.max);

    let words = "the cat and The dog and THE end".split(' ').map(String::from).collect::<Vec<_>>();
    let mut wc = pick_normalizer("lower").then(Counter { counts: BTreeMap::new() });
    let running = run_all(&mut wc, words.clone());
    println!("{} {:?}", wc.name(), running);
    println!("{:?}", wc.b.counts);

    for kind in ["upper", "trim", "other"] {
        let mut n = pick_normalizer(kind);
        let out = run_all(&mut n, vec!["  Hi ".to_string(), "   ".to_string()]);
        println!("{:<12} {:?}", n.name(), out);
    }

    let mut dynchain: Vec<DynStage<i64, i64>> = vec![
        Box::new(map("inc", |x: i64| x + 1)),
        Box::new(filter(|x: &i64| x % 2 == 0)),
        Box::new(map("half", |x: i64| x / 2)),
    ];
    let results: Vec<Option<i64>> = (0..6).map(|x| {
        let mut v = Some(x);
        for s in dynchain.iter_mut() { v = v.and_then(|y| s.process(y)); }
        v
    }).collect();
    println!("{:?}", results);
    println!("{:?}", dynchain.iter().map(|s| s.name()).collect::<Vec<_>>());

    let mut pf = Parse::<f64>(PhantomData).then(map("fmt", |x: f64| format!("{:.2}", x)));
    println!("{} {:?}", pf.name(), run_all(&mut pf, vec!["3.14159".into(), "nan?".into(), "2".into()]));
}
