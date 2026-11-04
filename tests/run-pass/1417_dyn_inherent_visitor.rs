use std::any::Any;
use std::fmt::Debug;

trait Shape: Debug {
    fn area(&self) -> f64;
    fn boxed(self) -> Box<dyn Shape> where Self: Sized + 'static { Box::new(self) }
    fn as_any(&self) -> &dyn Any;
    fn eq_dyn(&self, o: &dyn Shape) -> bool;
}
impl dyn Shape {
    fn double_area(&self) -> f64 { self.area() * 2.0 }
    fn is<T: Shape + 'static>(&self) -> bool { self.as_any().is::<T>() }
}
impl dyn Shape + Send {
    fn sendable(&self) -> &'static str { "send" }
}
impl PartialEq for dyn Shape {
    fn eq(&self, o: &Self) -> bool { self.eq_dyn(o) }
}
#[derive(Debug, PartialEq, Clone)]
struct Circle(f64);
#[derive(Debug, PartialEq, Clone)]
struct Rect(f64, f64);
impl Shape for Circle {
    fn area(&self) -> f64 { 3.0 * self.0 * self.0 }
    fn as_any(&self) -> &dyn Any { self }
    fn eq_dyn(&self, o: &dyn Shape) -> bool { o.as_any().downcast_ref::<Self>() == Some(self) }
}
impl Shape for Rect {
    fn area(&self) -> f64 { self.0 * self.1 }
    fn as_any(&self) -> &dyn Any { self }
    fn eq_dyn(&self, o: &dyn Shape) -> bool { o.as_any().downcast_ref::<Self>() == Some(self) }
}
impl<T: Shape + ?Sized> Shape for Box<T> {
    fn area(&self) -> f64 { (**self).area() + 0.5 }
    fn as_any(&self) -> &dyn Any { (**self).as_any() }
    fn eq_dyn(&self, o: &dyn Shape) -> bool { (**self).eq_dyn(o) }
}

fn total<S: Shape + ?Sized>(xs: &[&S]) -> f64 { xs.iter().map(|s| s.area()).sum() }

trait Node { fn accept<V: Visitor>(&self, v: &mut V) -> V::Out; }
trait Visitor { type Out; fn num(&mut self, n: i64) -> Self::Out; fn add(&mut self, a: &Expr, b: &Expr) -> Self::Out; fn neg(&mut self, a: &Expr) -> Self::Out; }
enum Expr { Num(i64), Add(Box<Expr>, Box<Expr>), Neg(Box<Expr>) }
impl Node for Expr {
    fn accept<V: Visitor>(&self, v: &mut V) -> V::Out {
        match self { Expr::Num(n) => v.num(*n), Expr::Add(a, b) => v.add(a, b), Expr::Neg(a) => v.neg(a) }
    }
}
struct Eval;
impl Visitor for Eval {
    type Out = i64;
    fn num(&mut self, n: i64) -> i64 { n }
    fn add(&mut self, a: &Expr, b: &Expr) -> i64 { a.accept(self) + b.accept(self) }
    fn neg(&mut self, a: &Expr) -> i64 { -a.accept(self) }
}
struct Print { depth: usize, max: usize }
impl Visitor for Print {
    type Out = String;
    fn num(&mut self, n: i64) -> String { n.to_string() }
    fn add(&mut self, a: &Expr, b: &Expr) -> String {
        self.depth += 1;
        self.max = self.max.max(self.depth);
        let s = format!("({} + {})", a.accept(self), b.accept(self));
        self.depth -= 1;
        s
    }
    fn neg(&mut self, a: &Expr) -> String { format!("-{}", a.accept(self)) }
}

trait SumItems: Iterator + Sized {
    fn sum_i64(self) -> i64 where Self::Item: Into<i64> { self.map(Into::into).sum() }
}
impl<I: Iterator> SumItems for I {}

struct Machine<'a> { state: i32, hooks: Vec<Box<dyn FnMut(&mut i32) -> bool + 'a>> }
impl<'a> Machine<'a> {
    fn run(&mut self) -> usize {
        let mut fired = 0;
        for h in self.hooks.iter_mut() { if h(&mut self.state) { fired += 1; } }
        fired
    }
}

fn main() {
    let shapes: Vec<Box<dyn Shape>> = vec![Circle(1.0).boxed(), Rect(2.0, 3.0).boxed(), Box::new(Box::new(Circle(2.0)))];
    for s in &shapes { println!("{:?} {} {} {}", s, s.area(), s.double_area(), s.is::<Circle>()); }
    println!("{}", shapes[0] == Circle(1.0).boxed());
    println!("{}", *shapes[0] == *shapes[1]);
    println!("{}", &*shapes[1] == &Rect(2.0, 3.0) as &dyn Shape);
    let snd: Box<dyn Shape + Send> = Box::new(Rect(1.0, 1.0));
    println!("{} {}", snd.sendable(), snd.area());
    let refs: Vec<&dyn Shape> = shapes.iter().map(|b| b.as_ref()).collect();
    println!("{}", total(&refs));
    println!("{}", total(&[&Circle(1.0), &Circle(0.5)]));
    println!("{}", total::<Box<dyn Shape>>(&[&shapes[1]]));

    let e = Expr::Add(Box::new(Expr::Num(3)), Box::new(Expr::Neg(Box::new(Expr::Add(Box::new(Expr::Num(1)), Box::new(Expr::Num(4)))))));
    let mut p = Print { depth: 0, max: 0 };
    println!("{} = {} (depth {})", e.accept(&mut p), e.accept(&mut Eval), p.max);

    println!("{} {}", vec![1i32, 2, 3].into_iter().sum_i64(), [7u8, 8].iter().copied().sum_i64());
    let mut it = (1..10).into_iter();
    let firsts: Vec<_> = it.by_ref().take(3).collect();
    let rest: i32 = it.sum();
    println!("{:?} {}", firsts, rest);

    let mut log = Vec::new();
    {
        let lr = &mut log;
        let mut m = Machine { state: 0, hooks: vec![] };
        m.hooks.push(Box::new(|s| { *s += 5; true }));
        m.hooks.push(Box::new(move |s| { lr.push(*s); *s > 3 }));
        m.hooks.push(Box::new(|s| { *s *= 2; false }));
        println!("{} {}", m.run(), m.run());
        println!("state {}", m.state);
    }
    println!("{:?}", log);
}
