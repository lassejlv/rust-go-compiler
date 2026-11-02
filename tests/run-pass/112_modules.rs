mod geometry {
    pub const UNIT: f64 = 1.0;

    #[derive(Debug, Clone, Copy)]
    pub struct Point {
        pub x: f64,
        pub y: f64,
    }

    impl Point {
        pub fn origin() -> Point {
            Point { x: 0.0, y: 0.0 }
        }
        pub fn dist(&self, o: &Point) -> f64 {
            helpers::hypot(self.x - o.x, self.y - o.y)
        }
    }

    mod helpers {
        pub fn hypot(a: f64, b: f64) -> f64 {
            (a * a + b * b).sqrt()
        }
    }

    pub mod shapes {
        use super::Point;

        pub struct Polygon {
            pub pts: Vec<Point>,
            name: String,
        }

        impl Polygon {
            pub fn new(name: &str, pts: Vec<Point>) -> Self {
                Polygon { pts, name: name.to_string() }
            }
            pub fn name(&self) -> &str {
                &self.name
            }
            pub fn perimeter(&self) -> f64 {
                let n = self.pts.len();
                (0..n).map(|i| self.pts[i].dist(&self.pts[(i + 1) % n])).sum()
            }
            pub fn area(&self) -> f64 {
                let n = self.pts.len();
                let s: f64 = (0..n)
                    .map(|i| {
                        let (a, b) = (self.pts[i], self.pts[(i + 1) % n]);
                        a.x * b.y - b.x * a.y
                    })
                    .sum();
                s.abs() / 2.0 * super::UNIT
            }
        }

        pub fn square(side: f64) -> Polygon {
            Polygon::new(
                "square",
                vec![Point { x: 0.0, y: 0.0 }, Point { x: side, y: 0.0 }, Point { x: side, y: side }, Point { x: 0.0, y: side }],
            )
        }
    }
}

mod counter {
    static mut CALLS: u32 = 0;

    pub fn tick() -> u32 {
        unsafe {
            CALLS += 1;
            CALLS
        }
    }

    pub mod nested {
        pub fn double_tick() -> u32 {
            super::tick();
            super::tick()
        }

        pub mod deeper {
            pub fn triple() -> u32 {
                crate::counter::tick();
                self::super::double_tick()
            }
        }
    }
}

mod text {
    pub fn shout(s: &str) -> String {
        format!("{}!", s.to_uppercase())
    }

    pub(crate) fn whisper(s: &str) -> String {
        format!("({})", s.to_lowercase())
    }

    pub enum Tone {
        Loud,
        Quiet,
    }

    pub fn say(s: &str, t: Tone) -> String {
        match t {
            Tone::Loud => shout(s),
            Tone::Quiet => self::whisper(s),
        }
    }
}

use geometry::shapes::{self, Polygon};
use geometry::Point;
use text::Tone;

fn main() {
    let a = Point::origin();
    let b = Point { x: 3.0, y: 4.0 };
    println!("dist = {}", a.dist(&b));
    let sq = shapes::square(2.0);
    println!("{}: perimeter {} area {}", sq.name(), sq.perimeter(), sq.area());
    let tri = Polygon::new("tri", vec![a, b, Point { x: 3.0, y: 0.0 }]);
    println!("{}: perimeter {} area {}", tri.name(), tri.perimeter(), tri.area());
    println!("points: {:?}", tri.pts);
    println!("tick {}", counter::tick());
    println!("double_tick {}", counter::nested::double_tick());
    println!("triple {}", counter::nested::deeper::triple());
    println!("{}", text::say("Hello", Tone::Loud));
    println!("{}", text::say("Hello", Tone::Quiet));
    println!("{}", text::whisper("DIRECT"));
    println!("UNIT = {}", geometry::UNIT);
}
