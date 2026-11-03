use std::io::{self, BufRead, BufReader, Cursor, Read, Seek, SeekFrom, Write};

struct Upper<W: Write> { inner: W, count: usize }
impl<W: Write> Write for Upper<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let up: Vec<u8> = buf.iter().map(|b| b.to_ascii_uppercase()).collect();
        self.count += buf.len();
        self.inner.write_all(&up)?;
        Ok(buf.len())
    }
    fn flush(&mut self) -> io::Result<()> { self.inner.flush() }
}

struct Repeat { byte: u8, left: usize }
impl Read for Repeat {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let n = buf.len().min(self.left).min(3);
        for b in &mut buf[..n] { *b = self.byte; }
        self.left -= n;
        Ok(n)
    }
}

fn consume(r: &mut dyn Read) -> io::Result<String> {
    let mut s = String::new();
    r.read_to_string(&mut s)?;
    Ok(s)
}

fn emit(w: &mut dyn Write, items: &[i32]) -> io::Result<()> {
    for (i, x) in items.iter().enumerate() {
        if i > 0 { w.write_all(b", ")?; }
        write!(w, "{}", x)?;
    }
    Ok(())
}

fn main() -> io::Result<()> {
    let mut c = Cursor::new(Vec::new());
    c.write_all(b"hello world")?;
    c.seek(SeekFrom::Start(6))?;
    c.write_all(b"WORLD!")?;
    println!("{:?} pos={}", String::from_utf8(c.get_ref().clone()).unwrap(), c.position());
    c.set_position(0);
    let mut first = [0u8; 5];
    c.read_exact(&mut first)?;
    println!("{:?}", std::str::from_utf8(&first).unwrap());
    let rest = consume(&mut c)?;
    println!("rest {:?}", rest);

    let data = "alpha 1\nbeta 22\ngamma 333\n";
    let r = BufReader::new(Cursor::new(data));
    let total: usize = r.lines().map(|l| l.unwrap().split(' ').nth(1).unwrap().parse::<usize>().unwrap()).sum();
    println!("total {}", total);
    let mut cur = Cursor::new(b"a,b,,c".to_vec());
    let mut parts = vec![];
    loop {
        let mut buf = vec![];
        let n = cur.read_until(b',', &mut buf)?;
        if n == 0 { break; }
        parts.push(String::from_utf8(buf).unwrap());
    }
    println!("{:?}", parts);
    let mut split: Vec<_> = Cursor::new("x;y;z").split(b';').map(|s| s.unwrap()).collect();
    split.reverse();
    println!("{:?}", split);

    let mut up = Upper { inner: Vec::new(), count: 0 };
    emit(&mut up, &[1, 2, 3])?;
    write!(up, " done")?;
    println!("{} {}", String::from_utf8(up.inner).unwrap(), up.count);

    let mut src = Repeat { byte: b'z', left: 10 };
    let mut sink = Vec::new();
    let n = io::copy(&mut src, &mut sink)?;
    println!("copied {} {:?}", n, String::from_utf8(sink).unwrap());
    let mut limited = Cursor::new("0123456789").take(4);
    println!("take {}", consume(&mut limited)?);
    let mut chained = Cursor::new("ab").chain(Cursor::new("cd"));
    println!("chain {}", consume(&mut chained)?);
    let bytes: Vec<u8> = Cursor::new("xyz").bytes().map(|b| b.unwrap()).collect();
    println!("{:?}", bytes);
    let mut empty = io::empty();
    println!("empty {:?}", consume(&mut empty)?);
    let mut sinkw = io::sink();
    println!("sink wrote {}", sinkw.write(b"ignored")?);
    let mut rep = io::repeat(7).take(3);
    let mut out = vec![];
    rep.read_to_end(&mut out)?;
    println!("{:?}", out);
    let mut writers: Vec<Box<dyn Write>> = vec![Box::new(Vec::<u8>::new()), Box::new(io::sink())];
    for w in writers.iter_mut() { w.write_all(b"x")?; }
    println!("boxed writers ok {}", writers.len());
    let mut arr = [0u8; 4];
    let mut sl: &mut [u8] = &mut arr;
    sl.write_all(b"ab")?;
    println!("slice write {:?} remaining {}", arr, 2);
    let mut rd: &[u8] = b"slice reader";
    let mut w4 = [0u8; 5];
    rd.read_exact(&mut w4)?;
    println!("{:?} left {:?}", std::str::from_utf8(&w4).unwrap(), std::str::from_utf8(rd).unwrap());
    Ok(())
}
