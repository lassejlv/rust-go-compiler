// Trait objects with associated types fixed in the dyn type (dyn Trait<Out = T>),
// associated consts used through generic code, and dyn Iterator<Item = ...> boxes.
trait Codec {
    type Out;
    const TAG: &'static str;
    fn encode(&self, input: &str) -> Self::Out;
}

struct Len;
struct Upper;
struct Sum;

impl Codec for Len {
    type Out = usize;
    const TAG: &'static str = "len";
    fn encode(&self, input: &str) -> usize {
        input.len()
    }
}
impl Codec for Upper {
    type Out = String;
    const TAG: &'static str = "upper";
    fn encode(&self, input: &str) -> String {
        input.to_uppercase()
    }
}
impl Codec for Sum {
    type Out = usize;
    const TAG: &'static str = "sum";
    fn encode(&self, input: &str) -> usize {
        input.bytes().map(|b| b as usize).sum()
    }
}

fn tagged<C: Codec>(c: &C, s: &str) -> String
where
    C::Out: std::fmt::Debug,
{
    format!("{}:{:?}", C::TAG, c.encode(s))
}

// dyn-safe projection of Codec without the const
trait DynCodec {
    type Out;
    fn enc(&self, s: &str) -> Self::Out;
}
impl<C: Codec> DynCodec for C {
    type Out = C::Out;
    fn enc(&self, s: &str) -> C::Out {
        self.encode(s)
    }
}

trait Limits {
    const MAX: u32;
    const MIN: u32 = 0;
    fn clamp_val(v: u32) -> u32 {
        v.max(Self::MIN).min(Self::MAX)
    }
}
struct Small;
struct Big;
impl Limits for Small {
    const MAX: u32 = 10;
}
impl Limits for Big {
    const MAX: u32 = 1000;
    const MIN: u32 = 100;
}

fn evens(limit: u32) -> Box<dyn Iterator<Item = u32>> {
    Box::new((0..limit).filter(|x| x % 2 == 0))
}

fn chained(a: Box<dyn Iterator<Item = u32>>, b: Box<dyn Iterator<Item = u32>>) -> Box<dyn Iterator<Item = u32>> {
    Box::new(a.chain(b).map(|x| x * 3))
}

fn main() {
    println!("{}", tagged(&Len, "hello"));
    println!("{}", tagged(&Upper, "héllo"));
    println!("{}", tagged(&Sum, "abc"));

    let counters: Vec<Box<dyn DynCodec<Out = usize>>> = vec![Box::new(Len), Box::new(Sum)];
    for c in &counters {
        println!("usize codec: {}", c.enc("xyz"));
    }
    let total: usize = counters.iter().map(|c| c.enc("ab")).sum();
    println!("total {}", total);

    let s: &dyn DynCodec<Out = String> = &Upper;
    println!("string codec: {}", s.enc("mixed Case"));

    for v in [0u32, 5, 50, 500, 5000] {
        println!("{} -> small {} big {}", v, Small::clamp_val(v), Big::clamp_val(v));
    }
    println!("consts: {} {} {} {}", Small::MIN, Small::MAX, Big::MIN, Big::MAX);

    let it = chained(evens(7), Box::new(vec![100, 200].into_iter()));
    let v: Vec<u32> = it.collect();
    println!("{:?}", v);

    let mut dyn_it: Box<dyn Iterator<Item = u32>> = evens(20);
    let first = dyn_it.next();
    let rest: Vec<u32> = dyn_it.by_ref().skip(2).take(3).collect();
    let after = dyn_it.next();
    println!("{:?} {:?} {:?}", first, rest, after);
    let hint = evens(10).size_hint();
    println!("hint {:?}", hint);
}
