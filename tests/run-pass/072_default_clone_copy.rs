#[derive(Debug, Default, Clone, PartialEq)]
struct Config {
    name: String,
    retries: u32,
    verbose: bool,
    ratio: f64,
    tags: Vec<String>,
    limit: Option<u32>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
struct Pixel {
    r: u8,
    g: u8,
    b: u8,
}

#[derive(Debug)]
struct Server {
    host: String,
    port: u16,
}

impl Default for Server {
    fn default() -> Self {
        Server { host: "localhost".to_string(), port: 8080 }
    }
}

#[derive(Debug, Default)]
enum Mode {
    #[default]
    Idle,
    Running,
}

fn brighten(mut p: Pixel) -> Pixel {
    p.r = p.r.saturating_add(50);
    p.g = p.g.saturating_add(50);
    p.b = p.b.saturating_add(50);
    p
}

fn main() {
    let c = Config::default();
    println!("{:?}", c);
    let c2 = Config { name: "prod".to_string(), retries: 3, ..Default::default() };
    println!("{:?}", c2);
    let mut c3 = c2.clone();
    c3.tags.push("cloned".to_string());
    c3.limit = Some(10);
    println!("c2 tags = {:?}, c3 tags = {:?}", c2.tags, c3.tags);
    println!("c2 == c3: {}", c2 == c3);
    let s = Server::default();
    println!("{:?}", s);
    let s2 = Server { port: 9000, ..Default::default() };
    println!("{}:{}", s2.host, s2.port);
    let m: Mode = Default::default();
    println!("mode = {:?}, other = {:?}", m, Mode::Running);
    let p = Pixel { r: 10, g: 220, b: 128 };
    let q = p;
    let bright = brighten(p);
    println!("p = {:?}", p);
    println!("q = {:?}", q);
    println!("bright = {:?}", bright);
    println!("default pixel = {:?}", Pixel::default());
    let pixels = [p; 3];
    println!("array of copies: {}", pixels.len());
    let ints: (i32, f64, bool, char, String) = Default::default();
    println!("defaults: {:?}", ints);
    let v: Vec<i32> = Default::default();
    let o: Option<String> = Default::default();
    println!("vec default = {:?}, option default = {:?}", v, o);
    let original = vec![String::from("a"), String::from("b")];
    let mut copy = original.clone();
    copy[0].push('!');
    println!("original = {:?}, copy = {:?}", original, copy);
}
