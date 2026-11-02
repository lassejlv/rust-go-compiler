// A simplified flexbox layout engine (taffy-like): main-axis sizing with flex-grow/shrink,
// justify-content, align-items, gaps, padding, and nested containers.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Dir {
    Row,
    Column,
}
#[derive(Clone, Copy, Debug)]
enum Justify {
    Start,
    End,
    Center,
    SpaceBetween,
    SpaceAround,
}
#[derive(Clone, Copy, Debug)]
enum Align {
    Start,
    Center,
    End,
    Stretch,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct Rect {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
}

struct NodeStyle {
    dir: Dir,
    justify: Justify,
    align: Align,
    gap: f32,
    padding: f32,
    basis: Option<(f32, f32)>, // preferred (w, h)
    grow: f32,
    shrink: f32,
}

impl Default for NodeStyle {
    fn default() -> Self {
        NodeStyle { dir: Dir::Row, justify: Justify::Start, align: Align::Stretch, gap: 0.0, padding: 0.0, basis: None, grow: 0.0, shrink: 1.0 }
    }
}

struct Node {
    name: &'static str,
    style: NodeStyle,
    children: Vec<Node>,
    layout: Rect,
}

fn leaf(name: &'static str, w: f32, h: f32, grow: f32) -> Node {
    Node { name, style: NodeStyle { basis: Some((w, h)), grow, ..Default::default() }, children: vec![], layout: Rect::default() }
}

fn container(name: &'static str, style: NodeStyle, children: Vec<Node>) -> Node {
    Node { name, style, children, layout: Rect::default() }
}

fn main_cross(dir: Dir, w: f32, h: f32) -> (f32, f32) {
    match dir {
        Dir::Row => (w, h),
        Dir::Column => (h, w),
    }
}

// intrinsic size of a node (content-based) used as flex basis for containers
fn intrinsic(n: &Node) -> (f32, f32) {
    if let Some(b) = n.style.basis {
        return b;
    }
    let mut main = 0.0f32;
    let mut cross = 0.0f32;
    for c in &n.children {
        let (cw, ch) = intrinsic(c);
        let (m, x) = main_cross(n.style.dir, cw, ch);
        main += m;
        cross = cross.max(x);
    }
    if n.children.len() > 1 {
        main += n.style.gap * (n.children.len() - 1) as f32;
    }
    let p2 = 2.0 * n.style.padding;
    let (w, h) = match n.style.dir {
        Dir::Row => (main, cross),
        Dir::Column => (cross, main),
    };
    (w + p2, h + p2)
}

fn layout(n: &mut Node, rect: Rect) {
    n.layout = rect;
    if n.children.is_empty() {
        return;
    }
    let s = &n.style;
    let inner = Rect { x: rect.x + s.padding, y: rect.y + s.padding, w: rect.w - 2.0 * s.padding, h: rect.h - 2.0 * s.padding };
    let (avail_main, avail_cross) = main_cross(s.dir, inner.w, inner.h);
    let count = n.children.len() as f32;
    let gaps = s.gap * (count - 1.0);
    let bases: Vec<(f32, f32)> = n.children.iter().map(|c| {
        let (w, h) = intrinsic(c);
        main_cross(s.dir, w, h)
    }).collect();
    let used: f32 = bases.iter().map(|b| b.0).sum::<f32>() + gaps;
    let free = avail_main - used;
    let mut sizes: Vec<f32> = bases.iter().map(|b| b.0).collect();
    if free > 0.0 {
        let total_grow: f32 = n.children.iter().map(|c| c.style.grow).sum();
        if total_grow > 0.0 {
            for (i, c) in n.children.iter().enumerate() {
                sizes[i] += free * c.style.grow / total_grow;
            }
        }
    } else if free < 0.0 {
        let total_shrink: f32 = n.children.iter().zip(&bases).map(|(c, b)| c.style.shrink * b.0).sum();
        if total_shrink > 0.0 {
            for (i, c) in n.children.iter().enumerate() {
                sizes[i] += free * c.style.shrink * bases[i].0 / total_shrink;
                sizes[i] = sizes[i].max(0.0);
            }
        }
    }
    let used_now: f32 = sizes.iter().sum::<f32>() + gaps;
    let remaining = (avail_main - used_now).max(0.0);
    let (mut pos, between) = match s.justify {
        Justify::Start => (0.0, s.gap),
        Justify::End => (remaining, s.gap),
        Justify::Center => (remaining / 2.0, s.gap),
        Justify::SpaceBetween => {
            if count > 1.0 {
                (0.0, s.gap + remaining / (count - 1.0))
            } else {
                (0.0, s.gap)
            }
        }
        Justify::SpaceAround => {
            let each = remaining / count;
            (each / 2.0, s.gap + each)
        }
    };
    let dir = s.dir;
    let align = s.align;
    for (i, c) in n.children.iter_mut().enumerate() {
        let main_size = sizes[i];
        let cross_size = match align {
            Align::Stretch => avail_cross,
            _ => bases[i].1.min(avail_cross),
        };
        let cross_off = match align {
            Align::Start | Align::Stretch => 0.0,
            Align::Center => (avail_cross - cross_size) / 2.0,
            Align::End => avail_cross - cross_size,
        };
        let r = match dir {
            Dir::Row => Rect { x: inner.x + pos, y: inner.y + cross_off, w: main_size, h: cross_size },
            Dir::Column => Rect { x: inner.x + cross_off, y: inner.y + pos, w: cross_size, h: main_size },
        };
        layout(c, r);
        pos += main_size + between;
    }
}

fn dump(n: &Node, depth: usize) {
    let r = n.layout;
    println!("{}{:<10} x={:<6.1} y={:<6.1} w={:<6.1} h={:.1}", "  ".repeat(depth), n.name, r.x, r.y, r.w, r.h);
    for c in &n.children {
        dump(c, depth + 1);
    }
}

fn build(justify: Justify, align: Align) -> Node {
    container(
        "row",
        NodeStyle { dir: Dir::Row, justify, align, gap: 10.0, padding: 5.0, ..Default::default() },
        vec![leaf("a", 50.0, 20.0, 0.0), leaf("b", 30.0, 40.0, 0.0), leaf("c", 20.0, 10.0, 0.0)],
    )
}

fn main() {
    for (j, a) in [
        (Justify::Start, Align::Stretch),
        (Justify::End, Align::Start),
        (Justify::Center, Align::Center),
        (Justify::SpaceBetween, Align::End),
        (Justify::SpaceAround, Align::Center),
    ] {
        let mut n = build(j, a);
        layout(&mut n, Rect { x: 0.0, y: 0.0, w: 300.0, h: 60.0 });
        println!("-- {:?} / {:?}", j, a);
        dump(&n, 0);
    }

    let mut app = container(
        "window",
        NodeStyle { dir: Dir::Column, gap: 4.0, ..Default::default() },
        vec![
            leaf("titlebar", 0.0, 24.0, 0.0),
            container(
                "body",
                NodeStyle { dir: Dir::Row, grow: 1.0, gap: 2.0, ..Default::default() },
                vec![leaf("sidebar", 120.0, 0.0, 0.0), leaf("editor", 200.0, 0.0, 3.0), leaf("panel", 100.0, 0.0, 1.0)],
            ),
            leaf("status", 0.0, 20.0, 0.0),
        ],
    );
    layout(&mut app, Rect { x: 0.0, y: 0.0, w: 640.0, h: 480.0 });
    println!("-- app");
    dump(&app, 0);

    // shrinking when content overflows
    let mut tight = container(
        "tight",
        NodeStyle { dir: Dir::Row, ..Default::default() },
        vec![leaf("x", 100.0, 10.0, 0.0), leaf("y", 300.0, 10.0, 0.0), leaf("z", 100.0, 10.0, 0.0)],
    );
    tight.children[0].style.shrink = 0.0;
    layout(&mut tight, Rect { x: 0.0, y: 0.0, w: 400.0, h: 10.0 });
    println!("-- shrink");
    dump(&tight, 0);
}
