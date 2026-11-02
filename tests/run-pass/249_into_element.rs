// IntoElement / AnyElement conversions: impl IntoElement for String, &'static str, numbers, Option,
// custom components (RenderOnce), and type-erased AnyElement children in a builder.
use std::fmt::Write;

trait Element {
    fn render(&self, out: &mut String, indent: usize);
}

struct AnyElement(Box<dyn Element>);

impl AnyElement {
    fn render(&self, out: &mut String, indent: usize) {
        self.0.render(out, indent)
    }
}

trait IntoElement: Sized {
    type Element: Element + 'static;
    fn into_element(self) -> Self::Element;
    fn into_any_element(self) -> AnyElement {
        AnyElement(Box::new(self.into_element()))
    }
}

struct TextElement(String);
impl Element for TextElement {
    fn render(&self, out: &mut String, indent: usize) {
        writeln!(out, "{}\"{}\"", " ".repeat(indent), self.0).unwrap();
    }
}

impl IntoElement for &'static str {
    type Element = TextElement;
    fn into_element(self) -> TextElement {
        TextElement(self.to_string())
    }
}
impl IntoElement for String {
    type Element = TextElement;
    fn into_element(self) -> TextElement {
        TextElement(self)
    }
}

macro_rules! impl_into_element_for_numbers {
    ($($t:ty),*) => {
        $(impl IntoElement for $t {
            type Element = TextElement;
            fn into_element(self) -> TextElement { TextElement(self.to_string()) }
        })*
    };
}
impl_into_element_for_numbers!(i32, u64, f32, usize);

struct Empty;
impl Element for Empty {
    fn render(&self, _: &mut String, _: usize) {}
}

enum Either<A, B> {
    A(A),
    B(B),
}
impl<A: Element, B: Element> Element for Either<A, B> {
    fn render(&self, out: &mut String, indent: usize) {
        match self {
            Either::A(a) => a.render(out, indent),
            Either::B(b) => b.render(out, indent),
        }
    }
}

impl<T: IntoElement> IntoElement for Option<T> {
    type Element = Either<T::Element, Empty>;
    fn into_element(self) -> Self::Element {
        match self {
            Some(t) => Either::A(t.into_element()),
            None => Either::B(Empty),
        }
    }
}

struct Div {
    tag: &'static str,
    classes: Vec<&'static str>,
    children: Vec<AnyElement>,
}

fn div() -> Div {
    Div { tag: "div", classes: vec![], children: vec![] }
}

impl Div {
    fn class(mut self, c: &'static str) -> Self {
        self.classes.push(c);
        self
    }
    fn child(mut self, c: impl IntoElement) -> Self {
        self.children.push(c.into_any_element());
        self
    }
    fn children<I>(mut self, it: I) -> Self
    where
        I: IntoIterator,
        I::Item: IntoElement,
    {
        self.children.extend(it.into_iter().map(IntoElement::into_any_element));
        self
    }
    fn when(self, cond: bool, f: impl FnOnce(Self) -> Self) -> Self {
        if cond {
            f(self)
        } else {
            self
        }
    }
}

impl Element for Div {
    fn render(&self, out: &mut String, indent: usize) {
        let cls = if self.classes.is_empty() { String::new() } else { format!(" .{}", self.classes.join(".")) };
        writeln!(out, "{}<{}{}>", " ".repeat(indent), self.tag, cls).unwrap();
        for c in &self.children {
            c.render(out, indent + 2);
        }
    }
}
impl IntoElement for Div {
    type Element = Div;
    fn into_element(self) -> Div {
        self
    }
}

// A RenderOnce-like component converting itself into an element tree.
trait RenderOnce {
    fn render(self) -> Div;
}
struct Component<C: RenderOnce>(C);
impl<C: RenderOnce + 'static> IntoElement for Component<C> {
    type Element = Div;
    fn into_element(self) -> Div {
        self.0.render()
    }
}

struct Button {
    label: &'static str,
    disabled: bool,
}
impl RenderOnce for Button {
    fn render(self) -> Div {
        Div { tag: "button", classes: vec![], children: vec![] }
            .class("btn")
            .when(self.disabled, |d| d.class("disabled"))
            .child(self.label)
    }
}

struct ListItem {
    idx: usize,
    text: String,
}
impl RenderOnce for ListItem {
    fn render(self) -> Div {
        div().class("item").child(self.idx).child(self.text)
    }
}

fn main() {
    let user: Option<&'static str> = Some("ada");
    let missing: Option<String> = None;
    let items = vec!["alpha", "beta", "gamma"];
    let tree = div()
        .class("root")
        .child("static str")
        .child(String::from("owned string"))
        .child(42)
        .child(3.5f32)
        .child(user)
        .child(missing)
        .child(Component(Button { label: "OK", disabled: false }))
        .child(Component(Button { label: "Cancel", disabled: true }))
        .child(
            div()
                .class("list")
                .children(items.iter().enumerate().map(|(i, s)| Component(ListItem { idx: i, text: s.to_uppercase() })))
                .when(items.len() > 2, |d| d.child("…and more")),
        )
        .children(vec![1u64, 2, 3])
        .children(Vec::<String>::new());
    let mut out = String::new();
    tree.into_any_element().render(&mut out, 0);
    print!("{}", out);
    println!("lines={}", out.lines().count());
}
