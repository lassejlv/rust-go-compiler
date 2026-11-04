// io::Cursor, Read::take/chain/bytes, BufRead::fill_buf/consume, io::copy, sink/empty/repeat,
// Write for Vec/Cursor<&mut [u8]>, LineWriter, BufWriter::into_inner, custom Read/Write impls.
use std::io::{self, BufRead, BufReader, BufWriter, Cursor, Read, Seek, SeekFrom, Write};

struct Upper<W: Write>(W);
impl<W: Write> Write for Upper<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let n = buf.len().min(3); // short writes
        let up: Vec<u8> = buf[..n].iter().map(|b| b.to_ascii_uppercase()).collect();
        self.0.write_all(&up)?;
        Ok(n)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.0.flush()
    }
}

struct Counter {
    n: u8,
}
impl Read for Counter {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.n >= 10 || buf.is_empty() {
            return Ok(0);
        }
        buf[0] = b'0' + self.n;
        self.n += 1;
        Ok(1)
    }
}

fn main() -> io::Result<()> {
    let mut c = Cursor::new(b"hello\nworld\nthird line".to_vec());
    let mut line = String::new();
    let n = c.read_line(&mut line)?;
    println!("{} {:?} pos {}", n, line, c.position());
    c.seek(SeekFrom::Current(2))?;
    let mut rest = String::new();
    c.read_to_string(&mut rest)?;
    println!("{:?}", rest);
    println!("eof read_line {}", c.read_line(&mut line)?);
    c.set_position(0);
    let words: Vec<String> = c.lines().map(|l| l.unwrap()).collect();
    println!("{:?}", words);

    let a: &[u8] = b"abc";
    let b: &[u8] = b"defgh";
    let mut chained = a.chain(b).take(6);
    let mut s = String::new();
    chained.read_to_string(&mut s)?;
    println!("chain+take {:?}", s);
    let bytes: Vec<u8> = Counter { n: 0 }.bytes().map(|b| b.unwrap()).collect();
    println!("{:?}", String::from_utf8(bytes).unwrap());
    let mut exact = [0u8; 4];
    Counter { n: 3 }.read_exact(&mut exact)?;
    println!("exact {:?}", std::str::from_utf8(&exact).unwrap());
    let err = Counter { n: 8 }.read_exact(&mut exact).unwrap_err();
    println!("read_exact eof {:?}", err.kind());

    let mut br = BufReader::with_capacity(4, Counter { n: 0 });
    let first = br.fill_buf()?.to_vec();
    br.consume(1);
    let second = br.fill_buf()?.to_vec();
    println!("fill_buf {:?} {:?} buffer {:?}", first, second, br.buffer());
    let mut tail = Vec::new();
    br.read_to_end(&mut tail)?;
    println!("tail {:?}", String::from_utf8(tail).unwrap());

    let mut out = Vec::new();
    let copied = io::copy(&mut Cursor::new("copy me please"), &mut Upper(&mut out))?;
    println!("copied {} -> {:?}", copied, String::from_utf8_lossy(&out));
    let mut u = Upper(Vec::new());
    write!(u, "fmt {} {:>4}", 1, "x")?;
    println!("{:?}", String::from_utf8(u.0).unwrap());

    let mut backing = [0u8; 8];
    {
        let mut cw = Cursor::new(&mut backing[..]);
        cw.write_all(b"12345")?;
        let r = cw.write_all(b"6789");
        println!("overflow err {:?} pos {}", r.unwrap_err().kind(), cw.position());
    }
    println!("{:?}", backing);
    let mut vc = Cursor::new(Vec::new());
    vc.write_all(b"abcdef")?;
    vc.seek(SeekFrom::Start(2))?;
    vc.write_all(b"XY")?;
    vc.seek(SeekFrom::End(3))?;
    vc.write_all(b"!")?;
    println!("{:?}", vc.into_inner());

    println!("sink {}", io::copy(&mut io::repeat(b'z').take(1000), &mut io::sink())?);
    let mut e = String::new();
    io::empty().read_to_string(&mut e)?;
    println!("empty {:?}", e);

    let mut bw = BufWriter::with_capacity(16, Vec::new());
    bw.write_all(b"buffered")?;
    println!("buffered len {} inner {}", bw.buffer().len(), bw.get_ref().len());
    bw.write_all(b" and more than sixteen")?;
    let inner = bw.into_inner().map_err(|e| e.into_error())?;
    println!("{:?}", String::from_utf8(inner).unwrap());
    let mut lw = io::LineWriter::new(Vec::new());
    lw.write_all(b"partial")?;
    println!("linewriter before nl {}", lw.get_ref().len());
    lw.write_all(b" line\nnext")?;
    println!("after nl {:?}", String::from_utf8_lossy(lw.get_ref()));
    let mut sp = BufReader::new(Cursor::new("k1=v1;k2=v2;;k3"));
    let mut buf = Vec::new();
    while sp.read_until(b';', &mut buf)? > 0 {
        print!("[{}]", String::from_utf8_lossy(&buf));
        buf.clear();
    }
    println!();
    let mut stdout = io::stdout().lock();
    writeln!(stdout, "locked stdout {}", 1)?;
    let e = io::Error::new(io::ErrorKind::InvalidData, "custom bad");
    println!("{} {:?}", e, e.kind());
    let e2 = io::Error::other("other err");
    println!("{} {:?} {}", e2, e2.kind(), io::Error::from(io::ErrorKind::NotFound));
    Ok(())
}
