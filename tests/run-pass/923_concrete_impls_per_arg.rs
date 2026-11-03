use std::fmt::Debug;

#[derive(Debug, Clone)]
struct Tagged<T> { tag: &'static str, v: T }

trait Render { fn render(&self) -> String; }
impl Render for Tagged<i32> { fn render(&self) -> String { format!("{}:int:{}", self.tag, self.v) } }
impl Render for Tagged<String> { fn render(&self) -> String { format!("{}:str:{:?}", self.tag, self.v) } }
impl Render for Tagged<Vec<i32>> { fn render(&self) -> String { format!("{}:vec:{}", self.tag, self.v.len()) } }
impl<T: Render> Render for Tagged<Option<T>> {
    fn render(&self) -> String {
        match &self.v { Some(t) => format!("{}:some[{}]", self.tag, t.render()), None => format!("{}:none", self.tag) }
    }
}
impl Render for i32 { fn render(&self) -> String { format!("i{}", self) } }
impl<T: Render> Render for Tagged<(T, T)> {
    fn render(&self) -> String { format!("{}:pair({},{})", self.tag, self.v.0.render(), self.v.1.render()) }
}

impl Tagged<i32> { fn bump(&mut self) { self.v += 1; } }
impl Tagged<String> { fn bump(&mut self) { self.v.push('!'); } }
impl<T: Debug> Tagged<Vec<T>> { fn bump(&mut self) where T: Clone { if let Some(l) = self.v.last().cloned() { self.v.push(l); } } }

trait Kind { fn kind() -> &'static str; }
impl Kind for u8 { fn kind() -> &'static str { "byte" } }
impl Kind for String { fn kind() -> &'static str { "string" } }
impl<T: Kind> Kind for Vec<T> { fn kind() -> &'static str { "vec" } }
impl<T: Kind> Kind for Option<T> { fn kind() -> &'static str { T::kind() } }
impl<A: Kind, B: Kind> Kind for (A, B) { fn kind() -> &'static str { if A::kind() == B::kind() { "homog" } else { "hetero" } } }

fn kinds<T: Kind, U: Kind>() -> String { format!("{}/{}", T::kind(), U::kind()) }

fn render_all(items: &[&dyn Render]) -> Vec<String> { items.iter().map(|r| r.render()).collect() }

fn main() {
    let mut a = Tagged { tag: "a", v: 41 };
    let mut b = Tagged { tag: "b", v: String::from("hi") };
    let mut c = Tagged { tag: "c", v: vec![1, 2] };
    a.bump(); b.bump(); c.bump(); c.bump();
    let d = Tagged { tag: "d", v: Some(Tagged { tag: "inner", v: 7 }) };
    let e: Tagged<Option<Tagged<String>>> = Tagged { tag: "e", v: None };
    let f = Tagged { tag: "f", v: (3, 4) };
    let g = Tagged { tag: "g", v: (Tagged { tag: "x", v: 1 }, Tagged { tag: "y", v: 2 }) };
    for s in render_all(&[&a, &b, &c, &d, &e, &f, &g]) { println!("{}", s); }
    let mut sv = Tagged { tag: "sv", v: vec!["q"] };
    sv.bump();
    println!("{:?}", sv);
    println!("{}", kinds::<u8, String>());
    println!("{}", kinds::<Vec<u8>, Option<String>>());
    println!("{} {}", <(u8, Option<u8>)>::kind(), <(u8, String) as Kind>::kind());
}
