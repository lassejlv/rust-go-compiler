use std::collections::BTreeMap;
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
enum Shape {
    Circle { r: f64 },
    Rect(u32, u32),
    Empty,
}

#[derive(Debug)]
struct Scene {
    name: &'static str,
    shapes: Vec<Shape>,
    tags: BTreeMap<u8, Option<&'static str>>,
    origin: (i32, i32),
    unit: (),
    nested: Option<Box<Scene>>,
}

struct Hexy(u32);
impl fmt::Display for Hexy {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if f.alternate() {
            write!(f, "Hexy[{:#x}]", self.0)
        } else if let Some(p) = f.precision() {
            write!(f, "H{:.*}", p, self.0 as f64)
        } else {
            f.pad(&format!("H{}", self.0))
        }
    }
}

struct Padded<'a>(&'a str);
impl fmt::Display for Padded<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.pad_integral(true, "0x", self.0)
    }
}

fn main() {
    let x = 1234.5678f64;
    println!("[{:>+#010.3e}]", x);
    println!("[{:<+12.2e}]", -x);
    println!("[{:^15.1E}]", x);
    println!("[{:e}] [{:E}] [{:e}]", 0.0f64, 1e-7f64, 123456u32);
    println!("[{:+}] [{:+}] [{:+.0}]", 0.0f64, -0.0f64, 2.5f64);
    println!("[{:08.2}] [{:+08.2}] [{:08}]", -3.14159, 3.14159, -42);
    println!("[{:#010x}] [{:#06b}] [{:#o}] [{:#X}]", 255, 5, 8, 0xabcdefu32);
    println!("[{:x}] [{:X}] [{:b}]", -1i8, -2i16, -1i8);
    println!("[{:10.3}] [{:<10}] [{:^10}] [{:>10}]", "abcdef", "ab", "ab", "ab");
    println!("[{:*^9}] [{:->7}] [{:_<5}]", "mid", 42, 'c');
    println!("[{:5}] [{:<5}] [{:^5}] [{:>5}]", true, 1.5, 'x', "é");
    let w = 8;
    let p = 2;
    println!("[{:w$.p$}] [{:>1$}] [{:.*}] [{val:^width$}]", 3.14159, 7, 3, 2.71828, val = "v", width = w);
    println!("[{0}-{0}-{1}] [{a}{b}{a}]", 1, 2, a = 'x', b = "y");
    println!("{{literal}} {{{}}}", 5);

    let s = Scene {
        name: "main",
        shapes: vec![Shape::Circle { r: 1.5 }, Shape::Rect(2, 3), Shape::Empty],
        tags: [(1, Some("one")), (2, None)].into_iter().collect(),
        origin: (-1, 2),
        unit: (),
        nested: Some(Box::new(Scene {
            name: "child",
            shapes: vec![],
            tags: BTreeMap::new(),
            origin: (0, 0),
            unit: (),
            nested: None,
        })),
    };
    println!("{:?}", s);
    println!("{:#?}", s);
    println!("{:x?} {:X?} {:#x?}", vec![10, 255], (171u8, 3000i32), Some(31));
    println!("{:5?}|{:<8?}|{:>+6?}", 1, Some(2), 3.0);
    println!("{:?} {:?} {:?}", "q\"uote\n\t\\", 'X', '\'');
    println!("{:?} {:?}", "\u{7f}\u{0}é\u{200b}", '\u{301}');
    println!("{:?}", (1, "a", [1.0, 2.5], ((),)));
    println!("{:#?}", (1, "a"));
    println!("{:#?}", Vec::<i32>::new());
    println!("{:#?}", Some(Shape::Rect(1, 1)));
    println!("{:.2?}", vec![1.23456, 2.0]);
    println!("{:?} {:?} {:?}", 1.0f64, 0.1f32, 1e21f64);
    println!("{:?} {:?}", f64::MIN_POSITIVE, 1e-7);

    println!("[{}] [{:#}] [{:.3}] [{:>8}] [{:<8}]", Hexy(255), Hexy(255), Hexy(3), Hexy(7), Hexy(7));
    println!("[{:>8}] [{:08}] [{:+}] [{:#}]", Padded("ff"), Padded("ff"), Padded("1"), Padded("2"));

    let v = format!("{:>width$}", "r", width = 4);
    println!("{}|{}", v, v.len());
    let mut out = String::new();
    use std::fmt::Write;
    write!(out, "{}-{:03}", "a", 7).unwrap();
    writeln!(out, "!").unwrap();
    print!("{}", out);
    eprintln!("this goes to stderr {}", 1);
    println!("{}", format_args!("{} {}", 1, 2));
    let name = "inline";
    let num = 3.0;
    println!("{name} {num:>6.2} {name:?} {num:e}");
    println!("{:?} {:?}", i64::MIN, u64::MAX);
    println!("{:e} {:e} {:E}", 1.0f32, 1234.5f32, 0.00012f64);
    println!("{:.0e} {:.1e} {:10.2e}|", 15.0, 0.25, 99999.0);
    println!("{:?}", std::cmp::Ordering::Less);
    println!("{:?} {:?}", Box::new(5), std::rc::Rc::new("rc"));
    println!("{:?}", std::cell::RefCell::new(vec![1]));
    println!("{:?}", std::cell::Cell::new(3));
    println!("{:?} {:?}", String::from("s"), &&&"r");
    println!("{:?}", [Some(1), None]);
    println!("{:?}", Ok::<u8, String>(1));
    println!("{:?}", 'a'..='z');
    println!("{:?} {:?} {:?}", ..3, 2.., (1..4).step_by(2));
}
