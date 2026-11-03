// Newer language features: trait upcasting coercions (dyn Sub -> dyn Super),
// async closures (AsyncFn traits), precise capturing `use<..>`, RPITIT
// (impl Trait in trait return position), and associated type bounds.
use std::fmt::Debug;
use std::future::Future;
use std::pin::{pin, Pin};
use std::task::{Context, Poll, Waker};

trait Named { fn name(&self) -> String; }
trait Animal: Named + Debug { fn sound(&self) -> &'static str; fn legs(&self) -> u32 { 4 } }
trait Pet: Animal { fn owner(&self) -> &str; }

#[derive(Debug)] struct Dog { owner: String }
#[derive(Debug)] struct Bird;

impl Named for Dog { fn name(&self) -> String { "dog".into() } }
impl Animal for Dog { fn sound(&self) -> &'static str { "woof" } }
impl Pet for Dog { fn owner(&self) -> &str { &self.owner } }
impl Named for Bird { fn name(&self) -> String { "bird".into() } }
impl Animal for Bird { fn sound(&self) -> &'static str { "tweet" } fn legs(&self) -> u32 { 2 } }

fn names(xs: &[&dyn Named]) -> Vec<String> { xs.iter().map(|x| x.name()).collect() }

trait Container {
    fn items(&self) -> impl Iterator<Item = u32> + '_;
    fn total(&self) -> u32 { self.items().sum() }
}
struct Evens(u32);
struct Listed(Vec<u32>);
impl Container for Evens { fn items(&self) -> impl Iterator<Item = u32> + '_ { (0..self.0).map(|x| x * 2) } }
impl Container for Listed { fn items(&self) -> impl Iterator<Item = u32> + '_ { self.0.iter().copied().rev() } }

fn first_debug<I>(it: I) -> String where I: IntoIterator<Item: Debug> {
    it.into_iter().next().map(|x| format!("{:?}", x)).unwrap_or_default()
}

fn counter<'a>(v: &'a [i32], k: i32) -> impl Iterator<Item = i32> + use<'a> {
    v.iter().map(move |x| x * k)
}

fn block_on<F: Future>(f: F) -> F::Output {
    let mut f = pin!(f);
    let mut cx = Context::from_waker(Waker::noop());
    loop {
        if let Poll::Ready(v) = f.as_mut().poll(&mut cx) { return v; }
    }
}

struct YieldOnce(bool);
impl Future for YieldOnce {
    type Output = ();
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        if self.0 { Poll::Ready(()) } else { self.0 = true; cx.waker().wake_by_ref(); Poll::Pending }
    }
}

async fn apply_all<F>(f: F, xs: &[u32]) -> Vec<u32> where F: AsyncFn(u32) -> u32 {
    let mut out = Vec::new();
    for &x in xs { out.push(f(x).await); }
    out
}

async fn run_mut<F: AsyncFnMut(&str)>(mut f: F) {
    f("one").await;
    f("two").await;
}

fn main() {
    let d = Dog { owner: "ann".into() };
    let b = Bird;
    let pet: &dyn Pet = &d;
    let animal: &dyn Animal = pet; // upcast
    let named: &dyn Named = animal; // upcast again
    println!("{} {} {} {}", pet.owner(), animal.sound(), named.name(), animal.legs());
    let zoo: Vec<Box<dyn Animal>> = vec![Box::new(Dog { owner: "bob".into() }), Box::new(Bird)];
    let as_named: Vec<&dyn Named> = zoo.iter().map(|a| a.as_ref() as &dyn Named).collect();
    println!("names: {:?}", names(&as_named));
    let boxed: Box<dyn Pet> = Box::new(Dog { owner: "cy".into() });
    let up: Box<dyn Animal> = boxed;
    println!("boxed upcast: {:?} legs={} {:?}", up, up.legs(), b);
    let rc: std::rc::Rc<dyn Animal> = std::rc::Rc::new(Bird);
    let rn: std::rc::Rc<dyn Named> = rc.clone();
    println!("rc upcast: {} strong={}", rn.name(), std::rc::Rc::strong_count(&rc));

    println!("rpitit: {} {} {:?}", Evens(5).total(), Listed(vec![1, 2, 3]).total(), Listed(vec![7, 8]).items().collect::<Vec<_>>());
    println!("assoc bound: {} {:?}", first_debug(vec![Some('x')]), first_debug(Vec::<u8>::new()));
    let data = vec![1, 2, 3];
    let c: Vec<i32> = counter(&data, 4).collect();
    println!("use<>: {:?}", c);

    let base = 10;
    let add = async |x: u32| { YieldOnce(false).await; x + base };
    println!("async closure: {:?}", block_on(apply_all(add, &[1, 2, 3])));
    let mut seen = Vec::new();
    block_on(run_mut(async |s: &str| { YieldOnce(false).await; seen.push(s.len()); }));
    println!("async fnmut captured: {:?}", seen);
    let fut = async { let a = async { 20 }.await; let b = async move { a + 1 }.await; a * b };
    println!("nested async blocks: {}", block_on(fut));
}
