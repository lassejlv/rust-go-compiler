// Clone vs Copy: moves vs copies, clone_from, custom Clone with side effects, Copy in closures/arrays/structs
use std::cell::Cell;

thread_local! {
    static CLONES: Cell<u32> = const { Cell::new(0) };
}

#[derive(Debug)]
struct Tracked {
    id: u32,
    data: Vec<u8>,
}
impl Clone for Tracked {
    fn clone(&self) -> Self {
        CLONES.with(|c| c.set(c.get() + 1));
        Tracked { id: self.id + 100, data: self.data.clone() }
    }
    fn clone_from(&mut self, src: &Self) {
        CLONES.with(|c| c.set(c.get() + 10));
        self.id = src.id + 1000;
        self.data.clear();
        self.data.extend_from_slice(&src.data);
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct P {
    x: i32,
    y: i32,
}

#[derive(Debug, Clone)]
struct Wrapper<T> {
    inner: T,
    label: String,
}

fn consume<T: std::fmt::Debug>(t: T) -> String {
    format!("{:?}", t)
}

fn clones() -> u32 {
    CLONES.with(|c| c.get())
}

fn main() {
    let p = P { x: 1, y: 2 };
    let mut q = p;
    q.x = 10;
    println!("{:?} {:?}", p, q);
    let arr = [p; 3];
    let mut arr2 = arr;
    arr2[0].y = 99;
    println!("{:?} {:?}", arr[0], arr2[0]);
    let mut pts = vec![p, q];
    for mut pt in pts.iter().copied() {
        pt.x += 1000;
        print!("{:?} ", pt);
    }
    println!();
    for pt in pts.iter_mut() {
        pt.y -= 1;
    }
    println!("{:?}", pts);
    let add = move |d: i32| P { x: p.x + d, y: p.y };
    println!("{:?} {:?} {:?}", add(1), add(2), p);

    let t = Tracked { id: 1, data: vec![1, 2] };
    let t2 = t.clone();
    println!("{} {} clones={}", t.id, t2.id, clones());
    let mut t3 = Tracked { id: 0, data: vec![9; 10] };
    t3.clone_from(&t);
    println!("{} {:?} clones={}", t3.id, t3.data, clones());
    let v = vec![t, t2];
    let v2 = v.clone();
    println!("{:?} clones={}", v2.iter().map(|t| t.id).collect::<Vec<_>>(), clones());
    let mut v3 = vec![Tracked { id: 5, data: vec![] }];
    v3.clone_from(&v);
    println!("{:?} clones={}", v3.iter().map(|t| t.id).collect::<Vec<_>>(), clones());
    let o: Option<Tracked> = Some(Tracked { id: 7, data: vec![] });
    let o2 = o.clone();
    println!("{:?} clones={}", o2.map(|t| t.id), clones());
    let w = Wrapper { inner: Tracked { id: 50, data: vec![3] }, label: "w".into() };
    let w2 = w.clone();
    println!("{} {} {} clones={}", w.inner.id, w2.inner.id, w2.label, clones());
    let moved = w;
    println!("{}", consume(moved.label));
    let s1 = String::from("hello");
    let s2 = s1.clone();
    let s3 = s1;
    println!("{} {}", s2, s3);
    let tup = (1, String::from("a"), P { x: 0, y: 0 });
    let (n, s, pp) = tup.clone();
    println!("{} {} {:?} {:?}", n, s, pp, tup);
    let refs: Vec<&P> = pts.iter().collect();
    let owned: Vec<P> = refs.iter().map(|&&p| p).collect();
    let owned2: Vec<P> = refs.into_iter().cloned().collect();
    println!("{} {}", owned == owned2, owned.len());
    let b = Box::new(P { x: 5, y: 5 });
    let b2 = b.clone();
    println!("{:?} {:?}", *b, b2);
    let r = &p;
    let copied = *r;
    println!("{:?}", copied);
    let big = [Tracked { id: 1, data: vec![] }, Tracked { id: 2, data: vec![] }];
    let before = clones();
    let big2 = big.clone();
    println!("{:?} delta={}", big2.iter().map(|t| t.id).collect::<Vec<_>>(), clones() - before);
    let f = |x: i32| x + 1;
    let g = f;
    println!("{} {}", f(1), g(2));
    let names = vec![String::from("a")];
    let h = move || names.len();
    let h2 = h.clone();
    println!("{} {}", h(), h2());
    let cow: std::borrow::Cow<[i32]> = std::borrow::Cow::Borrowed(&[1, 2, 3]);
    let mut cow2 = cow.clone();
    cow2.to_mut().push(4);
    println!("{:?} {:?}", cow, cow2);
}
