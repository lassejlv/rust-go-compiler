struct Color(u8, u8, u8);
struct Meters(f64);
struct Feet(f64);
struct Unit;

#[derive(Debug, Clone, Copy, PartialEq)]
struct Vec2(i32, i32);

impl Color {
    fn to_hex(&self) -> String {
        format!("#{:02x}{:02x}{:02x}", self.0, self.1, self.2)
    }
    fn invert(&self) -> Color {
        Color(255 - self.0, 255 - self.1, 255 - self.2)
    }
}

impl Meters {
    fn to_feet(&self) -> Feet {
        Feet(self.0 * 3.28084)
    }
}

impl Unit {
    fn describe(&self) -> &'static str {
        "I am a unit struct"
    }
}

impl Vec2 {
    fn add(self, o: Vec2) -> Vec2 {
        Vec2(self.0 + o.0, self.1 + o.1)
    }
}

fn main() {
    let red = Color(255, 0, 0);
    println!("red = {}", red.to_hex());
    let teal = Color(0, 128, 128);
    println!("teal = {}, inverted = {}", teal.to_hex(), teal.invert().to_hex());
    let Color(r, g, b) = teal;
    println!("components: {} {} {}", r, g, b);
    let m = Meters(100.0);
    let f = m.to_feet();
    println!("{} m = {:.2} ft", m.0, f.0);
    let u = Unit;
    println!("{}", u.describe());
    let a = Vec2(1, 2);
    let b = Vec2(10, 20);
    let c = a.add(b);
    println!("{:?} + {:?} = {:?}", a, b, c);
    println!("eq: {}", c == Vec2(11, 22));
    let vs = [Vec2(1, 1), Vec2(2, 3), Vec2(-1, 4)];
    let mut total = Vec2(0, 0);
    for v in vs {
        total = total.add(v);
    }
    println!("total = {:?}", total);
}
