use std::any::Any;
use std::fmt::{self, Debug, Display};

// Supertrait hierarchy with default methods and as_any downcasting, like GPUI's Render/Element hierarchy.
trait Component: Debug {
    fn id(&self) -> u32;
    fn kind(&self) -> &'static str {
        "component"
    }
    fn as_any(&self) -> &dyn Any;
}

trait Widget: Component + Display {
    fn width(&self) -> u32;
    fn render(&self) -> String {
        format!("[{}#{} w={}] {}", self.kind(), self.id(), self.width(), self)
    }
}

trait Focusable: Widget {
    fn focused(&self) -> bool;
    fn focus_ring(&self) -> String {
        if self.focused() {
            format!("*{}*", self.render())
        } else {
            self.render()
        }
    }
}

#[derive(Debug)]
struct Label {
    id: u32,
    text: String,
}

#[derive(Debug)]
struct Input {
    id: u32,
    value: String,
    focused: bool,
}

impl Component for Label {
    fn id(&self) -> u32 {
        self.id
    }
    fn kind(&self) -> &'static str {
        "label"
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}
impl Display for Label {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.text)
    }
}
impl Widget for Label {
    fn width(&self) -> u32 {
        self.text.len() as u32
    }
}

impl Component for Input {
    fn id(&self) -> u32 {
        self.id
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}
impl Display for Input {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "<input value={:?}>", self.value)
    }
}
impl Widget for Input {
    fn width(&self) -> u32 {
        20
    }
}
impl Focusable for Input {
    fn focused(&self) -> bool {
        self.focused
    }
}

fn render_all(ws: &[Box<dyn Widget>]) {
    for w in ws {
        println!("{}", w.render());
        if let Some(l) = w.as_any().downcast_ref::<Label>() {
            println!("  (a label with text {:?})", l.text);
        }
    }
}

fn upcast(f: &dyn Focusable) -> &dyn Widget {
    f
}

// Generic code requiring the supertrait chain
fn widest<W: Widget>(ws: &[W]) -> Option<&W> {
    ws.iter().max_by_key(|w| w.width())
}

// Supertrait with generic parameter
trait Codec<T>: Sized {
    fn encode(&self, t: &T) -> Vec<u8>;
    fn decode(&self, b: &[u8]) -> Option<T>;
    fn roundtrip(&self, t: &T) -> Option<T> {
        self.decode(&self.encode(t))
    }
}

trait NamedCodec<T>: Codec<T> {
    const NAME: &'static str;
    fn describe(&self, t: &T) -> String
    where
        T: Debug,
    {
        format!("{}({:?}) -> {:?}", Self::NAME, t, self.encode(t))
    }
}

struct U16Be;
impl Codec<u16> for U16Be {
    fn encode(&self, t: &u16) -> Vec<u8> {
        t.to_be_bytes().to_vec()
    }
    fn decode(&self, b: &[u8]) -> Option<u16> {
        Some(u16::from_be_bytes(b.try_into().ok()?))
    }
}
impl NamedCodec<u16> for U16Be {
    const NAME: &'static str = "u16be";
}

struct Utf8;
impl Codec<String> for Utf8 {
    fn encode(&self, t: &String) -> Vec<u8> {
        t.as_bytes().to_vec()
    }
    fn decode(&self, b: &[u8]) -> Option<String> {
        String::from_utf8(b.to_vec()).ok()
    }
}
impl NamedCodec<String> for Utf8 {
    const NAME: &'static str = "utf8";
}

fn main() {
    let ws: Vec<Box<dyn Widget>> = vec![
        Box::new(Label { id: 1, text: "Hello".into() }),
        Box::new(Input { id: 2, value: "typed".into(), focused: true }),
        Box::new(Label { id: 3, text: "Longer label".into() }),
    ];
    render_all(&ws);

    let inp = Input { id: 9, value: "".into(), focused: true };
    println!("{}", inp.focus_ring());
    let unf = Input { id: 10, value: "x".into(), focused: false };
    println!("{}", unf.focus_ring());
    let w: &dyn Widget = upcast(&inp);
    println!("upcast: {}", w.render());
    let c: &dyn Component = w;
    println!("component: {:?} kind={}", c, c.kind());

    let labels = vec![Label { id: 4, text: "a".into() }, Label { id: 5, text: "abcd".into() }];
    println!("widest: {:?}", widest(&labels).map(|l| l.id));

    println!("{}", U16Be.describe(&0xBEEF));
    println!("{:?}", U16Be.roundtrip(&513));
    println!("{}", Utf8.describe(&"hé".to_string()));
    println!("{:?}", Utf8.roundtrip(&"ok".into()));
    println!("{:?}", Utf8.decode(&[0xff, 0xfe]));
}
