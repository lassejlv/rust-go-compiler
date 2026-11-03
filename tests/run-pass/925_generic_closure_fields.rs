struct Machine<S, F, G>
where
    F: FnMut(&mut S, i32),
    G: Fn(&S) -> String,
{
    state: S,
    step: F,
    show: G,
    log: Vec<String>,
}

impl<S, F: FnMut(&mut S, i32), G: Fn(&S) -> String> Machine<S, F, G> {
    fn feed(&mut self, xs: &[i32]) -> &mut Self {
        for &x in xs {
            (self.step)(&mut self.state, x);
            let s = (self.show)(&self.state);
            self.log.push(s);
        }
        self
    }
}

fn machine<S, F: FnMut(&mut S, i32), G: Fn(&S) -> String>(state: S, step: F, show: G) -> Machine<S, F, G> {
    Machine { state, step, show, log: vec![] }
}

struct Callbacks<'a, T> {
    list: Vec<Box<dyn FnMut(&T) -> Option<String> + 'a>>,
}
impl<'a, T> Callbacks<'a, T> {
    fn on(&mut self, f: impl FnMut(&T) -> Option<String> + 'a) { self.list.push(Box::new(f)); }
    fn fire(&mut self, t: &T) -> Vec<String> { self.list.iter_mut().filter_map(|f| f(t)).collect() }
}

fn counter_gen<T: Clone + 'static>(items: Vec<T>) -> Box<dyn FnMut() -> Option<(usize, T)>> {
    let mut i = 0;
    Box::new(move || {
        let r = items.get(i).cloned().map(|x| (i, x));
        i += 1;
        r
    })
}

fn memoize<A: Copy + PartialEq, R: Copy>(f: impl Fn(A) -> R) -> impl FnMut(A) -> (R, bool) {
    let mut cache: Vec<(A, R)> = Vec::new();
    move |a| {
        if let Some(&(_, r)) = cache.iter().find(|(k, _)| *k == a) { return (r, true); }
        let r = f(a);
        cache.push((a, r));
        (r, false)
    }
}

fn main() {
    let mut m = machine(0i64, |s, x| *s = *s * 10 + x as i64, |s| format!("={}", s));
    m.feed(&[1, 2]).feed(&[3]);
    println!("{:?} {}", m.log, m.state);
    let mut total_calls = 0;
    let mut m2 = machine(Vec::<i32>::new(), |v, x| { total_calls += 1; if x % 2 == 0 { v.push(x) } }, |v| format!("{:?}", v));
    m2.feed(&[1, 2, 3, 4]);
    println!("{:?}", m2.log);
    drop(m2);
    println!("calls {}", total_calls);
    let mut seen = 0;
    {
        let mut cb: Callbacks<i32> = Callbacks { list: vec![] };
        cb.on(|x| if *x > 5 { Some(format!("big {}", x)) } else { None });
        cb.on(|x| { seen += 1; Some(format!("seen {}", x)) });
        for v in [3, 9] { println!("{:?}", cb.fire(&v)); }
    }
    println!("seen {}", seen);
    let mut g = counter_gen(vec!["a", "b"]);
    println!("{:?} {:?} {:?}", g(), g(), g());
    let mut sq = memoize(|x: u64| x * x);
    println!("{:?} {:?} {:?} {:?}", sq(3), sq(4), sq(3), sq(4));
}
