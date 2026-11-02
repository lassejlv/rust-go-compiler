// Manual Debug impls using the Formatter builder helpers, in normal and {:#?} modes.
use std::collections::BTreeMap;
use std::fmt;

struct Config {
    name: String,
    retries: u32,
    secret: String,
    tags: Vec<&'static str>,
}

impl fmt::Debug for Config {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Config")
            .field("name", &self.name)
            .field("retries", &self.retries)
            .field("tags", &self.tags)
            .finish_non_exhaustive()
    }
}

struct Handle(u32, &'static str);
impl fmt::Debug for Handle {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_tuple("Handle").field(&self.0).field(&format_args!("<{}>", self.1)).finish()
    }
}

struct Registry {
    entries: Vec<(String, Handle)>,
}
impl fmt::Debug for Registry {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_map().entries(self.entries.iter().map(|(k, v)| (k, v))).finish()
    }
}

struct Path(Vec<&'static str>);
impl fmt::Debug for Path {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut l = f.debug_list();
        for p in &self.0 {
            l.entry(p);
        }
        l.finish()
    }
}

struct Tags(Vec<u8>);
impl fmt::Debug for Tags {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_set().entries(self.0.iter().map(|b| *b as char)).finish()
    }
}

struct Opaque;
impl fmt::Debug for Opaque {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Opaque").finish_non_exhaustive()
    }
}

struct Unit;
impl fmt::Debug for Unit {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Unit").finish()
    }
}

struct Matrix {
    rows: Vec<Vec<i32>>,
}
impl fmt::Debug for Matrix {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut m = f.debug_map();
        for (i, r) in self.rows.iter().enumerate() {
            m.key(&format_args!("row{}", i)).value(r);
        }
        m.finish()
    }
}

#[derive(Debug)]
enum Node {
    Leaf(i32),
    Pair { left: Box<Node>, right: Box<Node> },
    Many(Vec<Node>),
    Empty,
}

#[derive(Debug, Default)]
struct Wrapper<'a> {
    label: &'a str,
    data: Option<(u8, char)>,
    map: BTreeMap<u8, &'a str>,
    unit: (),
    arr: [i8; 3],
}

fn main() {
    let c = Config { name: "srv".into(), retries: 3, secret: "hunter2".into(), tags: vec!["a", "b"] };
    let _ = &c.secret;
    println!("{:?}", c);
    println!("{:#?}", c);
    let r = Registry { entries: vec![("first".into(), Handle(1, "fd")), ("second".into(), Handle(22, "sock"))] };
    println!("{:?}", r);
    println!("{:#?}", r);
    println!("{:?} {:?} {:?} {:?}", Path(vec!["usr", "lib"]), Tags(b"xyz".to_vec()), Opaque, Unit);
    println!("{:#?}", Path(vec!["one"]));
    println!("{:#?}", Tags(vec![]));
    println!("{:?}", Matrix { rows: vec![vec![1, 2], vec![3]] });
    println!("{:#?}", Matrix { rows: vec![vec![1, 2]] });
    let tree = Node::Pair {
        left: Box::new(Node::Leaf(1)),
        right: Box::new(Node::Many(vec![Node::Leaf(2), Node::Empty])),
    };
    println!("{:?}", tree);
    println!("{:#?}", tree);
    let mut w = Wrapper::default();
    println!("{:?}", w);
    w.label = "lbl";
    w.data = Some((7, 'q'));
    w.map.insert(2, "two");
    w.map.insert(1, "one");
    w.arr = [-1, 0, 1];
    println!("{:#?}", w);
    println!("{:?}", (1, "two", [3.0, 4.5], Some(()), None::<u8>, Ok::<i32, ()>(5)));
    println!("{:?} {:?}", "esc\"aped\n\ttext\\", '\'');
    println!("{:?} {:?}", "unicode é ✓ \u{7f} \u{200b}", '\u{0}');
}
