// {:#?} pretty printing of nested structures: structs, tuple structs, unit structs, enums with
// all variant kinds, Vec/HashMap(sorted via BTreeMap)/Option/Box/tuples/arrays, empty collections,
// strings with escapes, and manual Debug impls using debug_struct/debug_list/debug_map/debug_tuple.
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

#[derive(Debug, Clone)]
struct Unit;

#[derive(Debug, Clone)]
struct Wrapper(i32, &'static str);

#[derive(Debug, Clone)]
enum Kind {
    A,
    B(u8),
    C { x: i32, y: Option<Box<Kind>> },
    D(Wrapper, Unit),
}

#[derive(Debug)]
struct Doc {
    title: String,
    tags: Vec<&'static str>,
    meta: BTreeMap<String, Kind>,
    empty: Vec<i32>,
    nothing: Option<()>,
    pair: (u8, char),
    grid: [[i8; 2]; 2],
    set: BTreeSet<u16>,
}

struct Matrix {
    rows: Vec<Vec<i32>>,
}

impl fmt::Debug for Matrix {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_list().entries(self.rows.iter().map(|r| Row(r))).finish()
    }
}
struct Row<'a>(&'a [i32]);
impl fmt::Debug for Row<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut t = f.debug_tuple("Row");
        for v in self.0 {
            t.field(v);
        }
        t.finish()
    }
}

struct Secret {
    user: String,
    password: String,
    attempts: u32,
}
impl fmt::Debug for Secret {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Secret")
            .field("user", &self.user)
            .field("password", &format_args!("***"))
            .field("attempts", &self.attempts)
            .finish_non_exhaustive()
    }
}

struct Registry(Vec<(&'static str, i32)>);
impl fmt::Debug for Registry {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_map().entries(self.0.iter().map(|(k, v)| (k, v))).finish()
    }
}

struct Bag(Vec<char>);
impl fmt::Debug for Bag {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_set().entries(&self.0).finish()
    }
}

fn main() {
    let mut meta = BTreeMap::new();
    meta.insert("first".to_string(), Kind::A);
    meta.insert("second".to_string(), Kind::B(7));
    meta.insert(
        "third".to_string(),
        Kind::C { x: -1, y: Some(Box::new(Kind::C { x: 2, y: None })) },
    );
    meta.insert("fourth".to_string(), Kind::D(Wrapper(5, "w\"q\""), Unit));
    let doc = Doc {
        title: "Line1\nLine2\t\"quoted\" \\ é".to_string(),
        tags: vec!["x", "y"],
        meta,
        empty: vec![],
        nothing: None,
        pair: (255, '\''),
        grid: [[1, -2], [3, -4]],
        set: [3, 1, 2].into_iter().collect(),
    };
    println!("{:?}", doc);
    println!("{:#?}", doc);

    let m = Matrix { rows: vec![vec![1, 2], vec![], vec![3]] };
    println!("{:?}", m);
    println!("{:#?}", m);

    let s = Secret { user: "root".into(), password: "hunter2".into(), attempts: 3 };
    println!("{:?}", s);
    println!("{:#?}", s);

    let r = Registry(vec![("one", 1), ("two", 2)]);
    println!("{:?}", r);
    println!("{:#?}", r);
    println!("{:?} {:#?}", Registry(vec![]), Bag(vec![]));
    println!("{:#?}", Bag(vec!['a', '\n']));

    let nested: Vec<Option<(i32, Vec<&str>)>> = vec![Some((1, vec!["a"])), None, Some((2, vec![]))];
    println!("{:#?}", nested);
    let unit_tuple = ((), (Unit,), Wrapper(0, ""));
    println!("{:?}", unit_tuple);
    println!("{:#?}", unit_tuple);
    let res: Result<Kind, String> = Err("bad".into());
    println!("{:#?}", res);
}
