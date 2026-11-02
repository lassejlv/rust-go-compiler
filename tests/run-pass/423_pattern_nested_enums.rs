// Deep matching on nested enums and structs: Box-free nesting through Vec/Option/tuples,
// enums inside enums, struct patterns with .. , constants in patterns, and exhaustive tuple matches.
#[derive(Debug, Clone, PartialEq)]
enum Shape {
    Circle { r: u32 },
    Rect { w: u32, h: u32 },
    Group(Vec<Shape>),
}

#[derive(Debug, Clone, PartialEq)]
enum Msg {
    Draw(Shape, Option<Color>),
    Move { id: u32, to: (i32, i32) },
    Batch(Vec<Msg>),
    Quit,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Color {
    Rgb(u8, u8, u8),
    Named(&'static str),
}

const ORIGIN: (i32, i32) = (0, 0);

fn color_name(c: &Option<Color>) -> String {
    match c {
        None => "default".into(),
        Some(Color::Rgb(0, 0, 0)) => "black".into(),
        Some(Color::Rgb(255, 255, 255)) => "white".into(),
        Some(Color::Rgb(r, 0, 0)) => format!("red{}", r),
        Some(Color::Rgb(r, g, b)) if r == g && g == b => format!("gray{}", r),
        Some(Color::Rgb(r, g, b)) => format!("#{:02x}{:02x}{:02x}", r, g, b),
        Some(Color::Named(n @ ("red" | "blue"))) => format!("primary {}", n),
        Some(Color::Named(n)) => n.to_string(),
    }
}

fn describe_shape(s: &Shape) -> String {
    match s {
        Shape::Circle { r: 0 } => "dot".into(),
        Shape::Circle { r } => format!("circle r={}", r),
        Shape::Rect { w, h } if w == h => format!("square {}", w),
        Shape::Rect { w: 1, .. } | Shape::Rect { h: 1, .. } => "line".into(),
        Shape::Rect { w, h } => format!("rect {}x{}", w, h),
        Shape::Group(v) if v.is_empty() => "empty group".into(),
        Shape::Group(v) => match v.as_slice() {
            [only] => format!("group of one ({})", describe_shape(only)),
            [Shape::Circle { .. }, rest @ ..] => format!("circle-led group +{}", rest.len()),
            [first, .., last] => format!("group {}..{}", describe_shape(first), describe_shape(last)),
            [] => unreachable!(),
        },
    }
}

fn handle(m: &Msg, depth: usize) -> Vec<String> {
    let pad = "  ".repeat(depth);
    match m {
        Msg::Quit => vec![format!("{}quit", pad)],
        Msg::Move { id, to: ORIGIN } => vec![format!("{}reset {}", pad, id)],
        Msg::Move { id: id @ 0..=9, to: (x, y) } if *x >= 0 && *y >= 0 => vec![format!("{}small id {} to Q1 ({},{})", pad, id, x, y)],
        Msg::Move { id, to: (x, _) } => vec![format!("{}move {} x={}", pad, id, x)],
        Msg::Draw(Shape::Group(inner), Some(Color::Named(n))) => {
            let mut v = vec![format!("{}group in {} with {} items", pad, n, inner.len())];
            for s in inner {
                v.push(format!("{}  - {}", pad, describe_shape(s)));
            }
            v
        }
        Msg::Draw(s, c) => vec![format!("{}draw {} in {}", pad, describe_shape(s), color_name(c))],
        Msg::Batch(ms) => {
            let mut out = vec![format!("{}batch[{}]", pad, ms.len())];
            for m in ms {
                out.extend(handle(m, depth + 1));
            }
            out
        }
    }
}

fn main() {
    let msgs = vec![
        Msg::Draw(Shape::Circle { r: 0 }, None),
        Msg::Draw(Shape::Rect { w: 4, h: 4 }, Some(Color::Rgb(0, 0, 0))),
        Msg::Draw(Shape::Rect { w: 1, h: 9 }, Some(Color::Rgb(200, 0, 0))),
        Msg::Draw(Shape::Rect { w: 3, h: 7 }, Some(Color::Rgb(9, 9, 9))),
        Msg::Draw(Shape::Rect { w: 9, h: 1 }, Some(Color::Rgb(1, 2, 254))),
        Msg::Draw(Shape::Group(vec![]), Some(Color::Named("blue"))),
        Msg::Draw(Shape::Group(vec![Shape::Rect { w: 2, h: 3 }]), Some(Color::Named("teal"))),
        Msg::Draw(Shape::Group(vec![Shape::Circle { r: 5 }, Shape::Circle { r: 6 }]), None),
        Msg::Draw(Shape::Group(vec![Shape::Rect { w: 2, h: 2 }, Shape::Circle { r: 1 }, Shape::Circle { r: 0 }]), Some(Color::Named("red"))),
        Msg::Move { id: 3, to: (0, 0) },
        Msg::Move { id: 3, to: (5, 6) },
        Msg::Move { id: 12, to: (5, 6) },
        Msg::Move { id: 4, to: (-1, 6) },
        Msg::Batch(vec![Msg::Quit, Msg::Batch(vec![Msg::Move { id: 1, to: (1, 1) }]), Msg::Draw(Shape::Circle { r: 2 }, Some(Color::Named("red")))]),
    ];
    for m in &msgs {
        for line in handle(m, 0) {
            println!("{}", line);
        }
    }
    let quits = msgs.iter().filter(|m| matches!(m, Msg::Batch(v) if v.contains(&Msg::Quit))).count();
    println!("batches with quit: {}", quits);
    let pair = (Some(3), Err::<i32, &str>("no"));
    let r = match pair {
        (Some(n), Ok(m)) => n + m,
        (Some(n), Err(e)) if e.len() < 3 => n * 100,
        (Some(_), Err(_)) => -1,
        (None, _) => 0,
    };
    println!("pair -> {}", r);
}
