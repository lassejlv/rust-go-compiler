// {:#?} pretty-printing of deeply nested structs, enums, tuples, maps, sets,
// Option/Result, unit/tuple structs, plus width/precision interplay with Debug.
use std::collections::{BTreeMap, BTreeSet, VecDeque};

#[derive(Debug, Clone, PartialEq)]
enum Shape {
    Unit,
    Circle { r: f64 },
    Poly(Vec<(i32, i32)>),
    Group(Box<Shape>, Box<Shape>),
}

#[derive(Debug)]
struct Unit;

#[derive(Debug)]
struct Wrapper(u8, &'static str);

#[derive(Debug)]
struct Layer {
    name: String,
    visible: bool,
    shapes: Vec<Shape>,
    tags: BTreeSet<&'static str>,
    meta: BTreeMap<String, Option<i64>>,
}

#[derive(Debug)]
struct Doc {
    title: &'static str,
    layers: Vec<Layer>,
    empty_vec: Vec<u8>,
    empty_map: BTreeMap<u8, u8>,
    unit: Unit,
    wrap: Wrapper,
    res: Result<(u8, char), String>,
    q: VecDeque<Option<Unit>>,
    nested: Option<Option<Box<Wrapper>>>,
}

fn main() {
    let mut meta = BTreeMap::new();
    meta.insert("z-order".to_string(), Some(-3));
    meta.insert("alpha".to_string(), None);
    let layer = Layer {
        name: "base \"layer\"".to_string(),
        visible: true,
        shapes: vec![
            Shape::Unit,
            Shape::Circle { r: 1.5 },
            Shape::Poly(vec![(0, 0), (1, -1)]),
            Shape::Group(Box::new(Shape::Circle { r: 0.0 }), Box::new(Shape::Poly(vec![]))),
        ],
        tags: ["b", "a", "c\n"].into_iter().collect(),
        meta,
    };
    let doc = Doc {
        title: "drawing",
        layers: vec![layer, Layer { name: String::new(), visible: false, shapes: vec![], tags: BTreeSet::new(), meta: BTreeMap::new() }],
        empty_vec: vec![],
        empty_map: BTreeMap::new(),
        unit: Unit,
        wrap: Wrapper(7, "w"),
        res: Ok((1, '\'')),
        q: VecDeque::from(vec![Some(Unit), None]),
        nested: Some(Some(Box::new(Wrapper(0, "")))),
    };
    println!("{:#?}", doc);
    println!("{:?}", doc.layers[0].shapes);
    let e: Result<(), String> = Err("bad\tthing".into());
    println!("{:#?}", e);
    println!("{:#?}", (1, "two", [3.0, 4.5], ()));
    println!("{:#?}", Some(Shape::Unit));
    println!("[{:>10?}] [{:<8?}] [{:^9?}]", Some(1), Shape::Unit, (1, 2));
    println!("[{:8.2?}] [{:+?}] [{:#x?}] [{:#X?}]", 3.14159f64, 5, vec![255, 16], (10u8, 171u8));
    println!("{:#?}", vec![vec![1, 2], vec![], vec![3]]);
    println!("{:?} {:?}", 'x', "q\"uote\\");
    println!("{:?} {:?} {:?}", f64::NAN, -0.0f64, 1e21f64);
    println!("{:?} {:?} {:?} {:?}", 1.0f32, 0.1f32 + 0.2f32, 1e-7f64, 123456789.0f64);
}
