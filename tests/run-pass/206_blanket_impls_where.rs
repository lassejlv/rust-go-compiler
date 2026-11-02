use std::fmt::{self, Debug, Display};
use std::rc::Rc;

// Extension trait with blanket impl over all Iterator types
trait IterExt: Iterator {
    fn sorted_vec(self) -> Vec<Self::Item>
    where
        Self: Sized,
        Self::Item: Ord,
    {
        let mut v: Vec<_> = self.collect();
        v.sort();
        v
    }

    fn join_with(self, sep: &str) -> String
    where
        Self: Sized,
        Self::Item: Display,
    {
        let mut s = String::new();
        for (i, x) in self.enumerate() {
            if i > 0 {
                s += sep;
            }
            s += &x.to_string();
        }
        s
    }
}

impl<I: Iterator> IterExt for I {}

// Blanket impl for references and smart pointers
trait Describe {
    fn describe(&self) -> String;
}

impl Describe for i32 {
    fn describe(&self) -> String {
        format!("int {}", self)
    }
}

impl Describe for str {
    fn describe(&self) -> String {
        format!("str {:?}", self)
    }
}

impl<T: Describe + ?Sized> Describe for &T {
    fn describe(&self) -> String {
        format!("&{}", (**self).describe())
    }
}

impl<T: Describe + ?Sized> Describe for Box<T> {
    fn describe(&self) -> String {
        format!("Box<{}>", (**self).describe())
    }
}

impl<T: Describe> Describe for Rc<T> {
    fn describe(&self) -> String {
        format!("Rc<{}>", (**self).describe())
    }
}

impl<T: Describe> Describe for Vec<T> {
    fn describe(&self) -> String {
        let parts: Vec<String> = self.iter().map(|x| x.describe()).collect();
        format!("[{}]", parts.join(", "))
    }
}

impl<T: Describe> Describe for Option<T> {
    fn describe(&self) -> String {
        match self {
            Some(x) => format!("Some({})", x.describe()),
            None => "Nothing".to_string(),
        }
    }
}

// Blanket impl from one trait to another
trait Named {
    fn name(&self) -> String;
}

trait Greet {
    fn greet(&self) -> String;
}

impl<T> Greet for T
where
    T: Named + Debug,
{
    fn greet(&self) -> String {
        format!("Hello, {} ({:?})", self.name(), self)
    }
}

#[derive(Debug)]
struct User {
    id: u32,
}

impl Named for User {
    fn name(&self) -> String {
        format!("user#{}", self.id)
    }
}

// Wrapper newtype with conditional Display impl
struct Wrapper<T>(Vec<T>);

impl<T: Display> Display for Wrapper<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "[")?;
        for (i, x) in self.0.iter().enumerate() {
            if i != 0 {
                write!(f, "; ")?;
            }
            write!(f, "{}", x)?;
        }
        write!(f, "]")
    }
}

impl<T> Wrapper<T>
where
    T: Clone + Into<f64>,
{
    fn mean(&self) -> f64 {
        let n = self.0.len() as f64;
        self.0.iter().cloned().map(Into::into).sum::<f64>() / n
    }
}

// Generic trait with blanket impl over closures
trait Handler<E> {
    fn handle(&self, e: &E) -> String;
}

impl<E, F> Handler<E> for F
where
    F: Fn(&E) -> String,
{
    fn handle(&self, e: &E) -> String {
        self(e)
    }
}

fn dispatch<E>(handlers: &[&dyn Handler<E>], e: E) {
    for h in handlers {
        println!("  {}", h.handle(&e));
    }
}

fn main() {
    println!("{:?}", vec![5, 3, 9, 1].into_iter().sorted_vec());
    println!("{}", "c a b".split(' ').sorted_vec().into_iter().join_with("/"));
    println!("{}", (1..=4).map(|x| x * x).join_with(" + "));

    let x = 7;
    println!("{}", x.describe());
    println!("{}", (&x).describe());
    println!("{}", (&&x).describe());
    println!("{}", "hi".describe());
    let b: Box<str> = "boxed".into();
    println!("{}", b.describe());
    println!("{}", Rc::new(3).describe());
    println!("{}", vec![Some(1), None, Some(3)].describe());
    println!("{}", Box::new(vec![Rc::new(4)]).describe());

    println!("{}", User { id: 42 }.greet());
    let w = Wrapper(vec![1u8, 2, 4]);
    println!("{} mean={}", w, w.mean());
    let wf = Wrapper(vec![1.5f32, 2.5]);
    println!("{} mean={}", wf, wf.mean());
    println!("{}", Wrapper(vec!["x", "y"]));

    let h1 = |e: &u32| format!("double {}", e * 2);
    let prefix = String::from("pfx");
    let h2 = move |e: &u32| format!("{}-{}", prefix, e);
    dispatch(&[&h1, &h2], 21);
}
