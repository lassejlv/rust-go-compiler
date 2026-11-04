mod i01 {
    use std::error::Error;
    use std::fmt::Debug;
    trait Shape: Debug { fn area(&self) -> f64; }
    #[derive(Debug)] struct Sq(f64);
    #[derive(Debug)] struct Ci(f64);
    impl Shape for Sq { fn area(&self) -> f64 { self.0 * self.0 } }
    impl Shape for Ci { fn area(&self) -> f64 { 3.0 * self.0 * self.0 } }
    struct Scene { main: Option<Box<dyn Shape>>, extra: Vec<Box<dyn Shape>>, cb: Option<Box<dyn Fn(f64) -> f64>> }
    fn err_of(s: &str) -> Option<Box<dyn Error>> { match s.parse::<i32>() { Ok(_) => None, Err(e) => Some(Box::new(e)) } }
    fn pick(n: u8) -> Box<dyn Shape> { if n == 0 { Box::new(Sq(1.0)) } else { Box::new(Ci(n as f64)) } }
    fn pairs() -> Vec<(String, Box<dyn Shape>)> { vec![("a".into(), Box::new(Sq(2.0))), ("b".into(), Box::new(Ci(1.0)))] }
    pub fn main() {
        let mut sc = Scene { main: None, extra: Vec::new(), cb: None };
        sc.main = Some(Box::new(Sq(3.0)));
        sc.extra.push(Box::new(Ci(1.0)));
        sc.extra.extend(vec![pick(0), pick(2)]);
        let k = 2.0;
        sc.cb = Some(Box::new(move |x| x * k));
        let total: f64 = sc.main.iter().chain(sc.extra.iter()).map(|s| s.area()).sum();
        println!("{} {:?}", total, sc.cb.as_ref().map(|f| f(total)));
        println!("{:?} {:?}", err_of("1").is_none(), err_of("z").map(|e| e.to_string()));
        for (n, s) in pairs() { println!("{} {:?} {}", n, s, s.area()); }
        let shapes: Vec<Box<dyn Shape>> = (0..3).map(|i| -> Box<dyn Shape> { if i % 2 == 0 { Box::new(Sq(i as f64)) } else { Box::new(Ci(i as f64)) } }).collect();
        println!("{:?}", shapes);
        let mut fs: Vec<Box<dyn Fn(i32) -> i32>> = Vec::new();
        for i in 0..3 { fs.push(Box::new(move |x| x + i)); }
        fs.push(Box::new(|x| x * 100));
        println!("{:?}", fs.iter().map(|f| f(1)).collect::<Vec<_>>());
        let maybe: Option<&dyn Shape> = sc.main.as_deref();
        println!("{:?}", maybe.map(|s| s.area()));
        let r: Result<Box<dyn Shape>, String> = Ok(Box::new(Sq(0.5)));
        println!("{:?}", r.map(|s| s.area()));
        let tuple: (Box<dyn Shape>, &dyn Debug) = (Box::new(Ci(0.1)), &5);
        println!("{:?} {:?}", tuple.0, tuple.1);
        let arr: [Box<dyn Fn() -> &'static str>; 2] = [Box::new(|| "x"), Box::new(|| "y")];
        println!("{}{}", arr[0](), arr[1]());
        let nested: Vec<Option<Box<dyn Shape>>> = vec![Some(Box::new(Sq(1.0))), None];
        println!("{}", nested.iter().flatten().count());
    }
}

mod j01 {
    use std::collections::HashMap;
    use std::convert::TryInto;
    fn takes_u32(x: u32) -> u32 { x }
    fn takes_vec_f64(v: Vec<f64>) -> f64 { v.iter().sum() }
    fn takes_arr(a: [u8; 4]) -> u32 { u32::from_be_bytes(a) }
    #[derive(Debug, Default)] struct Stats { count: usize, names: Vec<String>, map: HashMap<String, u32> }
    fn parse_all(s: &str) -> Result<Vec<u8>, std::num::ParseIntError> { let mut v = Vec::new(); for t in s.split(',') { v.push(t.trim().parse()?); } Ok(v) }
    pub fn main() {
        let v = vec![1u32, 2, 3];
        println!("{}", takes_u32(v.iter().sum()));
        println!("{}", takes_u32(v.iter().map(|x| x * 2).product()));
        println!("{}", takes_vec_f64(vec![1u8, 2].into_iter().map(Into::into).collect()));
        let bytes = vec![0u8, 0, 1, 2, 3];
        println!("{}", takes_arr(bytes[1..].try_into().unwrap()));
        let x: u16 = 300u32.try_into().unwrap();
        println!("{}", x);
        let s = "5";
        if s.parse() == Ok(5u8) { println!("five"); }
        let st = Stats { count: 2, ..Default::default() };
        println!("{:?}", st);
        println!("{:?} {:?}", parse_all("1, 2,3"), parse_all("1,x").is_err());
        let mut m = HashMap::new();
        m.insert("k", vec![]);
        m.get_mut("k").unwrap().push(1.5);
        println!("{:?}", m);
        let total = v.iter().fold(0, |a, b| a + b);
        println!("{}", takes_u32(total));
        let parsed: Vec<_> = "1 2 3".split(' ').map(|x| x.parse::<i64>()).collect::<Result<_, _>>().unwrap();
        println!("{:?}", parsed);
        let mut acc = Vec::new();
        acc.extend("ab".chars().map(|c| c as u32));
        println!("{}", takes_u32(acc[0] + acc[1]));
        let z = Default::default();
        let _: &str = z;
        println!("{:?}", z);
        let lookup: HashMap<_, _> = [(1, "one"), (2, "two")].into();
        println!("{:?}", lookup.get(&2));
        let empty = Vec::new();
        println!("{}", takes_vec_f64(empty));
        let n = "42".parse().unwrap_or(0);
        println!("{}", takes_u32(n));
        let pairs: Vec<(String, usize)> = vec!["aa", "b"].iter().map(|s| (s.to_string(), s.len())).collect();
        let (names, lens): (Vec<_>, Vec<_>) = pairs.into_iter().unzip();
        println!("{:?} {}", names, takes_u32(lens.iter().sum::<usize>() as u32));
        let max = [3.5, 1.0].iter().cloned().fold(f64::NAN, f64::max);
        println!("{}", max);
        let w: Box<[i32]> = vec![1, 2].into();
        let w2: Vec<i32> = w.into();
        println!("{:?}", w2);
        let c = std::iter::repeat("ab").take(2).collect::<String>();
        println!("{}", c);
    }
}

mod k01 {
    use std::io::{self, BufRead, BufReader, Read, Write};
    use std::fmt;
    use std::borrow::Borrow;
    use std::collections::{HashMap, HashSet, BinaryHeap};
    struct Rot13<R: Read> { inner: R }
    impl<R: Read> Read for Rot13<R> {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            let n = self.inner.read(buf)?;
            for b in &mut buf[..n] { *b = match *b { b'a'..=b'z' => (*b - b'a' + 13) % 26 + b'a', b'A'..=b'Z' => (*b - b'A' + 13) % 26 + b'A', o => o }; }
            Ok(n)
        }
    }
    struct Chunky<'a> { data: &'a [u8], pos: usize }
    impl Read for Chunky<'_> { fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> { let n = buf.len().min(3).min(self.data.len() - self.pos); buf[..n].copy_from_slice(&self.data[self.pos..self.pos + n]); self.pos += n; Ok(n) } }
    #[derive(Default)] struct Counter { bytes: usize, lines: usize, buf: Vec<u8> }
    impl Write for Counter {
        fn write(&mut self, b: &[u8]) -> io::Result<usize> { self.bytes += b.len(); self.lines += b.iter().filter(|&&c| c == b'\n').count(); self.buf.extend_from_slice(b); Ok(b.len()) }
        fn flush(&mut self) -> io::Result<()> { Ok(()) }
    }
    struct Upper(String);
    impl fmt::Write for Upper { fn write_str(&mut self, s: &str) -> fmt::Result { self.0.push_str(&s.to_uppercase()); Ok(()) } }
    #[derive(Debug, PartialEq, Eq, Hash)] struct Key(String);
    impl Borrow<str> for Key { fn borrow(&self) -> &str { &self.0 } }
    #[derive(Debug, PartialEq, Eq)] struct Task { pri: u8, name: &'static str }
    impl Ord for Task { fn cmp(&self, o: &Self) -> std::cmp::Ordering { o.pri.cmp(&self.pri).then_with(|| self.name.cmp(o.name)) } }
    impl PartialOrd for Task { fn partial_cmp(&self, o: &Self) -> Option<std::cmp::Ordering> { Some(self.cmp(o)) } }
    #[derive(Debug)] struct Celsius(f64);
    #[derive(Debug)] struct Fahrenheit(f64);
    impl PartialEq<Fahrenheit> for Celsius { fn eq(&self, o: &Fahrenheit) -> bool { (self.0 * 9.0 / 5.0 + 32.0 - o.0).abs() < 1e-9 } }
    pub fn main() -> io::Result<()> {
        let mut s = String::new();
        Rot13 { inner: "Hello, World!\nfoo".as_bytes() }.read_to_string(&mut s)?;
        println!("{:?}", s);
        let br = BufReader::with_capacity(4, Chunky { data: b"line one\nline two\nthree", pos: 0 });
        let lines: Vec<String> = br.lines().collect::<Result<_, _>>()?;
        println!("{:?}", lines);
        let mut c = Counter::default();
        writeln!(c, "a {}", 1)?;
        write!(&mut c, "b\nc")?;
        c.write_all(b"\n")?;
        println!("{} {} {:?}", c.bytes, c.lines, String::from_utf8_lossy(&c.buf));
        let mut u = Upper(String::new());
        use std::fmt::Write as _;
        write!(u, "x={} y={:?}", 5, "q").unwrap();
        println!("{}", u.0);
        let mut hm: HashMap<Key, i32> = HashMap::new();
        hm.insert(Key("alpha".into()), 1);
        println!("{:?} {:?}", hm.get("alpha"), hm.contains_key("beta"));
        let hs: HashSet<Key> = [Key("x".into())].into_iter().collect();
        println!("{}", hs.contains("x"));
        let mut heap = BinaryHeap::new();
        heap.push(Task { pri: 3, name: "c" });
        heap.push(Task { pri: 1, name: "z" });
        heap.push(Task { pri: 1, name: "a" });
        heap.push(Task { pri: 2, name: "b" });
        while let Some(t) = heap.pop() { print!("{}{} ", t.name, t.pri); }
        println!();
        println!("{} {}", Celsius(100.0) == Fahrenheit(212.0), Celsius(0.0) == Fahrenheit(33.0));
        let mut stdout = io::stdout().lock();
        stdout.write_all(b"raw bytes\n")?;
        Ok(())
    }
}

fn main() {
    println!("== i01");
    i01::main();
    println!("== j01");
    j01::main();
    println!("== k01");
    k01::main().unwrap();
}
