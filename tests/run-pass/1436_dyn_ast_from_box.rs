use std::collections::BTreeMap;
use std::fmt::{self, Debug, Display};
use std::rc::Rc;

trait Node: Debug { fn eval(&self, env: &BTreeMap<String, f64>) -> Result<f64, String>; fn children(&self) -> Vec<&dyn Node> { vec![] } }
#[derive(Debug)] struct Num(f64);
#[derive(Debug)] struct Var(String);
#[derive(Debug)] struct Bin { op: char, l: Box<dyn Node>, r: Box<dyn Node> }
impl Node for Num { fn eval(&self, _: &BTreeMap<String, f64>) -> Result<f64, String> { Ok(self.0) } }
impl Node for Var { fn eval(&self, env: &BTreeMap<String, f64>) -> Result<f64, String> { env.get(&self.0).copied().ok_or_else(|| format!("unbound {}", self.0)) } }
impl Node for Bin {
    fn eval(&self, env: &BTreeMap<String, f64>) -> Result<f64, String> {
        let (a, b) = (self.l.eval(env)?, self.r.eval(env)?);
        match self.op { '+' => Ok(a + b), '*' => Ok(a * b), '/' if b == 0.0 => Err("div by zero".into()), '/' => Ok(a / b), c => Err(format!("bad op {}", c)) }
    }
    fn children(&self) -> Vec<&dyn Node> { vec![&*self.l, &*self.r] }
}
impl<T: Node + 'static> From<T> for Box<dyn Node> { fn from(t: T) -> Self { Box::new(t) } }
impl From<f64> for Box<dyn Node> { fn from(x: f64) -> Self { Box::new(Num(x)) } }
impl From<&str> for Box<dyn Node> { fn from(s: &str) -> Self { Box::new(Var(s.to_string())) } }
fn bin(op: char, l: impl Into<Box<dyn Node>>, r: impl Into<Box<dyn Node>>) -> Bin { Bin { op, l: l.into(), r: r.into() } }
fn count(n: &dyn Node) -> usize { 1 + n.children().into_iter().map(count).sum::<usize>() }

trait Pretty { fn pretty(&self) -> String; }
impl<T: Debug + ?Sized> Pretty for T { fn pretty(&self) -> String { format!("{:?}", self) } }

struct Shown<'a>(&'a dyn Display);
impl Display for Shown<'_> { fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { write!(f, "<{}>", self.0) } }
fn join_all(xs: &[&dyn Display]) -> String { xs.iter().map(|x| Shown(*x).to_string()).collect::<Vec<_>>().join(" ") }

fn boxed_iter<'a, T: Clone + 'a>(v: &'a [T], rev: bool) -> Box<dyn Iterator<Item = T> + 'a> {
    if rev { Box::new(v.iter().rev().cloned()) } else { Box::new(v.iter().cloned()) }
}
fn rc_dyn(xs: Vec<Rc<dyn Node>>) -> Vec<Result<f64, String>> { let env = BTreeMap::new(); xs.iter().map(|n| n.eval(&env)).collect() }

fn main() {
    let e = bin('+', bin('*', 2.0, "x"), bin('/', "y", Num(4.0)));
    let mut env = BTreeMap::new();
    env.insert("x".to_string(), 3.0);
    println!("{:?}", e.eval(&env));
    env.insert("y".to_string(), 10.0);
    println!("{:?} nodes={}", e.eval(&env), count(&e));
    println!("{:?}", bin('/', 1.0, 0.0).eval(&env));
    println!("{:?}", bin('%', 1.0, 2.0).eval(&env));
    let b: Box<dyn Node> = Num(1.5).into();
    println!("{} {}", b.pretty(), (&*b).pretty());
    println!("{}", join_all(&[&1, &"two", &3.5, &'c']));
    println!("{:?} {:?}", boxed_iter(&[1, 2, 3], true).collect::<Vec<_>>(), boxed_iter(&["a"], false).collect::<Vec<_>>());
    let shared: Rc<dyn Node> = Rc::new(Num(2.0));
    println!("{:?}", rc_dyn(vec![shared.clone(), Rc::new(Var("q".into())), shared]));
}
