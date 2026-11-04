fn compose<A, B, C>(f: impl Fn(A) -> B, g: impl Fn(B) -> C) -> impl Fn(A) -> C { move |x| g(f(x)) }

fn layer<T: Copy + std::ops::Add<Output = T>>(x: T) -> impl Fn(T) -> T { move |y| y + x }

fn build() -> impl Fn(i64) -> i64 {
    let f = compose(layer(1), layer(2));
    let f = compose(f, layer(3));
    let f = compose(f, layer(4));
    let f = compose(f, layer(5));
    let f = compose(f, layer(6));
    let f = compose(f, layer(7));
    let f = compose(f, layer(8));
    let f = compose(f, |x| x * 2);
    let f = compose(f, layer(9));
    let f = compose(f, layer(10));
    let f = compose(f, layer(11));
    let f = compose(f, layer(12));
    let f = compose(f, |x| x - 1);
    let f = compose(f, layer(13));
    let f = compose(f, layer(14));
    let f = compose(f, layer(15));
    compose(f, layer(16))
}

fn nest<T: Clone + std::fmt::Debug>(t: T) -> String {
    let a = |x: T| {
        let b = |y: T| {
            let c = |z: T| {
                let d = |w: T| {
                    let e = |v: T| {
                        let f = |u: T| {
                            let g = |s: T| format!("{:?}", (s.clone(), s));
                            g(u)
                        };
                        f(v)
                    };
                    e(w)
                };
                d(z)
            };
            c(y)
        };
        b(x)
    };
    a(t)
}

fn gen_chain<I: Iterator<Item = u32> + Clone>(it: I) -> u32 {
    let a = it.clone().map(|x| x + 1).filter(|x| x % 2 == 0);
    let b = a.clone().zip(it.clone()).map(|(x, y)| x * y);
    let c = b.clone().chain(a.clone()).flat_map(|x| vec![x, x + 1]);
    let d = c.clone().skip_while(|x| *x < 3).take_while(|x| *x < 1000).enumerate().map(|(i, x)| i as u32 + x);
    let e = d.clone().zip(c.clone()).zip(b.clone()).map(|((p, q), r)| p + q + r);
    e.clone().sum::<u32>() + d.max().unwrap_or(0) + e.count() as u32
}

fn main() {
    let f = build();
    println!("{}", f(0));
    println!("{}", nest(1u8));
    println!("{}", nest("s"));
    println!("{}", nest(vec![Some(1.5)]));
    println!("{}", gen_chain(0..20u32));
    println!("{}", gen_chain(vec![5u32, 6, 7].into_iter()));
    println!("{}", gen_chain([1u32, 2].iter().copied()));
}
