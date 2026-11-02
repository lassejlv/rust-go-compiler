// PhantomData-typed units of measure and typed IDs: zero-sized marker types, generic impls
// gated on marker traits, conversions between units via trait consts, typestate for a connection,
// and size_of checks proving PhantomData is zero-sized.
use std::fmt;
use std::marker::PhantomData;
use std::ops::{Add, Mul, Sub};

trait Unit {
    const NAME: &'static str;
    const TO_BASE: f64;
}
#[derive(Debug, Clone, Copy)]
struct Meters;
#[derive(Debug, Clone, Copy)]
struct Feet;
#[derive(Debug, Clone, Copy)]
struct Miles;
impl Unit for Meters {
    const NAME: &'static str = "m";
    const TO_BASE: f64 = 1.0;
}
impl Unit for Feet {
    const NAME: &'static str = "ft";
    const TO_BASE: f64 = 0.3048;
}
impl Unit for Miles {
    const NAME: &'static str = "mi";
    const TO_BASE: f64 = 1609.344;
}

#[derive(Clone, Copy)]
struct Length<U: Unit> {
    v: f64,
    _u: PhantomData<U>,
}
impl<U: Unit> Length<U> {
    fn new(v: f64) -> Self {
        Length { v, _u: PhantomData }
    }
    fn to<V: Unit>(self) -> Length<V> {
        Length::new(self.v * U::TO_BASE / V::TO_BASE)
    }
}
impl<U: Unit> Add for Length<U> {
    type Output = Length<U>;
    fn add(self, o: Self) -> Self {
        Length::new(self.v + o.v)
    }
}
impl<U: Unit> Sub for Length<U> {
    type Output = Length<U>;
    fn sub(self, o: Self) -> Self {
        Length::new(self.v - o.v)
    }
}
impl<U: Unit> Mul<f64> for Length<U> {
    type Output = Length<U>;
    fn mul(self, k: f64) -> Self {
        Length::new(self.v * k)
    }
}
impl<U: Unit> fmt::Display for Length<U> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:.3}{}", self.v, U::NAME)
    }
}

struct Id<T> {
    raw: u32,
    _t: PhantomData<fn() -> T>,
}
impl<T> Clone for Id<T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T> Copy for Id<T> {}
impl<T> PartialEq for Id<T> {
    fn eq(&self, o: &Self) -> bool {
        self.raw == o.raw
    }
}
impl<T> fmt::Debug for Id<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let full = std::any::type_name::<T>();
        let short = full.rsplit("::").next().unwrap();
        write!(f, "Id<{}>({})", short, self.raw)
    }
}
struct User;
struct Order;

struct Disconnected;
struct Connected;
struct Authenticated;
struct Conn<S> {
    log: Vec<String>,
    _s: PhantomData<S>,
}
impl Conn<Disconnected> {
    fn new() -> Self {
        Conn { log: vec!["created".into()], _s: PhantomData }
    }
    fn connect(mut self, host: &str) -> Conn<Connected> {
        self.log.push(format!("connect {}", host));
        Conn { log: self.log, _s: PhantomData }
    }
}
impl Conn<Connected> {
    fn login(mut self, user: &str) -> Result<Conn<Authenticated>, Conn<Connected>> {
        if user.is_empty() {
            self.log.push("login failed".into());
            Err(self)
        } else {
            self.log.push(format!("login {}", user));
            Ok(Conn { log: self.log, _s: PhantomData })
        }
    }
}
impl Conn<Authenticated> {
    fn query(&mut self, q: &str) -> usize {
        self.log.push(format!("query {}", q));
        q.len()
    }
}
impl<S> Conn<S> {
    fn history(&self) -> String {
        self.log.join(" -> ")
    }
}

fn main() {
    let a: Length<Meters> = Length::new(100.0);
    let b: Length<Feet> = Length::new(328.084);
    let total = a + b.to::<Meters>();
    println!("{} + {} = {}", a, b, total);
    println!("{} = {}", total, total.to::<Miles>());
    println!("{}", (Length::<Miles>::new(1.0) - Length::new(0.5)).to::<Feet>() * 2.0);

    let u1: Id<User> = Id { raw: 7, _t: PhantomData };
    let u2 = u1;
    let o1: Id<Order> = Id { raw: 7, _t: PhantomData };
    println!("{:?} {:?} {:?} eq={}", u1, u2, o1, u1 == u2);

    println!("sizes: {} {} {} {}", std::mem::size_of::<Length<Miles>>(), std::mem::size_of::<Id<User>>(), std::mem::size_of::<PhantomData<String>>(), std::mem::size_of::<Conn<Connected>>() == std::mem::size_of::<Vec<String>>());

    let c = Conn::new().connect("db.local");
    let c = match c.login("") {
        Ok(_) => unreachable!(),
        Err(c) => c,
    };
    let mut c = c.login("admin").ok().unwrap();
    let n = c.query("SELECT 1");
    println!("{} ({} bytes)", c.history(), n);
}
