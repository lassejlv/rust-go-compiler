// std::mem: swap, replace, take, size_of(_val), align_of, discriminant, drop order with replace
use std::collections::HashMap;
use std::mem;

#[derive(Debug, Default)]
struct Buffer {
    data: Vec<u8>,
    name: String,
}

#[derive(Debug)]
enum State {
    Empty,
    Loaded(Vec<i32>),
    Done { total: i32 },
}

struct Noisy(&'static str);
impl Drop for Noisy {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

impl State {
    fn advance(&mut self) {
        *self = match mem::replace(self, State::Empty) {
            State::Empty => State::Loaded(vec![1, 2, 3]),
            State::Loaded(v) => State::Done { total: v.iter().sum() },
            d @ State::Done { .. } => d,
        };
    }
}

fn take_name(b: &mut Buffer) -> String {
    mem::take(&mut b.name)
}

fn main() {
    let mut a = vec![1, 2];
    let mut b = vec![9];
    mem::swap(&mut a, &mut b);
    println!("{:?} {:?}", a, b);
    let mut arr = [1, 2, 3, 4];
    let (l, r) = arr.split_at_mut(2);
    mem::swap(&mut l[0], &mut r[1]);
    println!("{:?}", arr);
    let mut s = String::from("old");
    let prev = mem::replace(&mut s, String::from("new"));
    println!("{} {}", prev, s);
    let mut buf = Buffer { data: vec![1, 2, 3], name: "buf".into() };
    let n = take_name(&mut buf);
    let d = mem::take(&mut buf.data);
    println!("{:?} {:?} {:?}", n, d, buf);
    let mut st = State::Empty;
    for _ in 0..4 {
        st.advance();
        println!("{:?}", st);
    }
    println!("{}", mem::discriminant(&State::Loaded(vec![])) == mem::discriminant(&State::Loaded(vec![5])));
    println!("{}", mem::discriminant(&State::Empty) == mem::discriminant(&st));
    let mut slot = Some(Noisy("first"));
    println!("replacing");
    let old = mem::replace(&mut slot, Some(Noisy("second")));
    println!("replaced");
    drop(old);
    println!("taking");
    let t = slot.take();
    println!("took {}", t.as_ref().map(|n| n.0).unwrap_or("-"));
    drop(t);
    let mut x = Noisy("x");
    let mut y = Noisy("y");
    mem::swap(&mut x, &mut y);
    println!("after swap x={} y={}", x.0, y.0);
    let mut v = vec![Noisy("v0"), Noisy("v1")];
    let v1 = mem::replace(&mut v[1], Noisy("v1b"));
    println!("got {}", v1.0);
    drop(v1);
    println!("{} {} {} {}", mem::size_of::<u8>(), mem::size_of::<(u8, u32)>(), mem::size_of::<[u16; 5]>(), mem::size_of::<Option<u32>>());
    println!("{} {} {}", mem::size_of::<Box<i32>>(), mem::size_of::<Option<Box<i32>>>(), mem::size_of::<&[u8]>());
    println!("{} {} {}", mem::size_of::<char>(), mem::size_of::<Option<char>>(), mem::size_of::<bool>());
    println!("{} {}", mem::size_of::<()>(), mem::size_of::<[u64; 0]>());
    println!("{} {}", mem::align_of::<u64>(), mem::align_of::<(u8, u16)>());
    let sv = [1u32, 2, 3];
    println!("{} {}", mem::size_of_val(&sv), mem::size_of_val(&sv[..2]));
    println!("{} {}", mem::size_of_val("héllo"), mem::size_of_val(&0u128));
    let dynv: &dyn std::fmt::Debug = &42u16;
    println!("{}", mem::size_of_val(dynv));
    let mut cache: HashMap<&str, Vec<i32>> = HashMap::new();
    cache.insert("k", vec![1, 2]);
    let taken = cache.get_mut("k").map(mem::take).unwrap_or_default();
    println!("{:?} {:?}", taken, cache["k"]);
    let mut opt = Some(3);
    let got = mem::take(&mut opt);
    println!("{:?} {:?}", got, opt);
    let mut nums = vec![5, 1, 4];
    let sorted = {
        let mut tmp = mem::take(&mut nums);
        tmp.sort();
        tmp
    };
    println!("{:?} {:?}", sorted, nums);
    println!("{}", mem::needs_drop::<String>() && !mem::needs_drop::<u32>());
    println!("end of main");
}
