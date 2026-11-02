// GPUI-like element tree: trait Element with request_layout / prepaint / paint phases,
// Box<dyn Element> children, a layout pass computing bounds, and a paint pass emitting a scene.
use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Default)]
struct Point {
    x: f32,
    y: f32,
}
#[derive(Clone, Copy, Debug, PartialEq, Default)]
struct Size {
    w: f32,
    h: f32,
}
#[derive(Clone, Copy, PartialEq, Default)]
struct Bounds {
    origin: Point,
    size: Size,
}
impl fmt::Debug for Bounds {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "[{},{} {}x{}]", self.origin.x, self.origin.y, self.size.w, self.size.h)
    }
}

#[derive(Debug)]
enum Primitive {
    Quad { bounds: Bounds, color: &'static str },
    Text { origin: Point, text: String },
    Border { bounds: Bounds },
}

#[derive(Default)]
struct Scene {
    prims: Vec<Primitive>,
    layer: usize,
}

struct LayoutCx {
    char_width: f32,
    line_height: f32,
}

trait Element {
    fn name(&self) -> String;
    fn request_layout(&mut self, cx: &LayoutCx, max_width: f32) -> Size;
    fn prepaint(&mut self, origin: Point);
    fn paint(&self, scene: &mut Scene);
    fn bounds(&self) -> Bounds;
    fn debug_tree(&self, depth: usize, out: &mut Vec<String>) {
        out.push(format!("{}{} {:?}", "  ".repeat(depth), self.name(), self.bounds()));
    }
}

struct Text {
    content: String,
    size: Size,
    bounds: Bounds,
}

impl Text {
    fn new(s: &str) -> Self {
        Text { content: s.into(), size: Size::default(), bounds: Bounds::default() }
    }
}

impl Element for Text {
    fn name(&self) -> String {
        format!("Text({:?})", self.content)
    }
    fn request_layout(&mut self, cx: &LayoutCx, max_width: f32) -> Size {
        let per_line = (max_width / cx.char_width).floor().max(1.0) as usize;
        let n = self.content.chars().count();
        let lines = ((n + per_line - 1) / per_line).max(1);
        let w = (n.min(per_line)) as f32 * cx.char_width;
        self.size = Size { w, h: lines as f32 * cx.line_height };
        self.size
    }
    fn prepaint(&mut self, origin: Point) {
        self.bounds = Bounds { origin, size: self.size };
    }
    fn paint(&self, scene: &mut Scene) {
        scene.prims.push(Primitive::Text { origin: self.bounds.origin, text: self.content.clone() });
    }
    fn bounds(&self) -> Bounds {
        self.bounds
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Axis {
    Row,
    Column,
}

struct Div {
    axis: Axis,
    gap: f32,
    padding: f32,
    bg: Option<&'static str>,
    border: bool,
    children: Vec<Box<dyn Element>>,
    child_sizes: Vec<Size>,
    size: Size,
    bounds: Bounds,
}

impl Div {
    fn new(axis: Axis) -> Self {
        Div { axis, gap: 0.0, padding: 0.0, bg: None, border: false, children: vec![], child_sizes: vec![], size: Size::default(), bounds: Bounds::default() }
    }
    fn child(mut self, c: impl Element + 'static) -> Self {
        self.children.push(Box::new(c));
        self
    }
    fn gap(mut self, g: f32) -> Self {
        self.gap = g;
        self
    }
    fn padding(mut self, p: f32) -> Self {
        self.padding = p;
        self
    }
    fn bg(mut self, c: &'static str) -> Self {
        self.bg = Some(c);
        self
    }
    fn border(mut self) -> Self {
        self.border = true;
        self
    }
}

impl Element for Div {
    fn name(&self) -> String {
        format!("Div({})", if self.axis == Axis::Row { "row" } else { "col" })
    }
    fn request_layout(&mut self, cx: &LayoutCx, max_width: f32) -> Size {
        let inner_max = max_width - 2.0 * self.padding;
        self.child_sizes.clear();
        let n = self.children.len();
        let mut main = 0.0f32;
        let mut cross = 0.0f32;
        let per_child = if self.axis == Axis::Row && n > 0 {
            (inner_max - self.gap * (n as f32 - 1.0)) / n as f32
        } else {
            inner_max
        };
        for c in self.children.iter_mut() {
            let s = c.request_layout(cx, per_child);
            self.child_sizes.push(s);
            match self.axis {
                Axis::Row => {
                    main += s.w;
                    cross = cross.max(s.h);
                }
                Axis::Column => {
                    main += s.h;
                    cross = cross.max(s.w);
                }
            }
        }
        if n > 1 {
            main += self.gap * (n as f32 - 1.0);
        }
        let (w, h) = if self.axis == Axis::Row { (main, cross) } else { (cross, main) };
        self.size = Size { w: w + 2.0 * self.padding, h: h + 2.0 * self.padding };
        self.size
    }
    fn prepaint(&mut self, origin: Point) {
        self.bounds = Bounds { origin, size: self.size };
        let mut cursor = Point { x: origin.x + self.padding, y: origin.y + self.padding };
        for (c, s) in self.children.iter_mut().zip(self.child_sizes.iter()) {
            c.prepaint(cursor);
            match self.axis {
                Axis::Row => cursor.x += s.w + self.gap,
                Axis::Column => cursor.y += s.h + self.gap,
            }
        }
    }
    fn paint(&self, scene: &mut Scene) {
        if let Some(color) = self.bg {
            scene.prims.push(Primitive::Quad { bounds: self.bounds, color });
        }
        scene.layer += 1;
        for c in &self.children {
            c.paint(scene);
        }
        if self.border {
            scene.prims.push(Primitive::Border { bounds: self.bounds });
        }
    }
    fn bounds(&self) -> Bounds {
        self.bounds
    }
    fn debug_tree(&self, depth: usize, out: &mut Vec<String>) {
        out.push(format!("{}{} {:?}", "  ".repeat(depth), self.name(), self.bounds));
        for c in &self.children {
            c.debug_tree(depth + 1, out);
        }
    }
}

fn draw(root: &mut dyn Element, width: f32) -> (Vec<String>, Scene) {
    let cx = LayoutCx { char_width: 8.0, line_height: 16.0 };
    root.request_layout(&cx, width);
    root.prepaint(Point::default());
    let mut scene = Scene::default();
    root.paint(&mut scene);
    let mut tree = vec![];
    root.debug_tree(0, &mut tree);
    (tree, scene)
}

fn main() {
    let mut root = Div::new(Axis::Column)
        .padding(4.0)
        .gap(2.0)
        .bg("white")
        .child(Text::new("Title bar"))
        .child(
            Div::new(Axis::Row)
                .gap(10.0)
                .border()
                .child(Text::new("left pane"))
                .child(Div::new(Axis::Column).bg("gray").child(Text::new("a")).child(Text::new("this text wraps around"))),
        )
        .child(Text::new("status"));
    for width in [400.0, 200.0] {
        let (tree, scene) = draw(&mut root, width);
        println!("== width {} ==", width);
        for l in tree {
            println!("{}", l);
        }
        println!("scene: {} prims, {} layers", scene.prims.len(), scene.layer);
        for p in &scene.prims {
            println!("  {:?}", p);
        }
    }
}
