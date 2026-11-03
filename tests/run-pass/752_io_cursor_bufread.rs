// In-memory I/O: Cursor with Read/Seek/BufRead (lines, split, read_until,
// read_line, read_exact, bytes, take, chain), Write into Vec<u8> and
// BufWriter, io::copy, LineWriter-ish formatting, and io::Error kinds.
use std::io::{self, BufRead, BufReader, BufWriter, Cursor, Read, Seek, SeekFrom, Write};

struct Upper<W: Write> { inner: W, written: usize }
impl<W: Write> Write for Upper<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let n = buf.len().min(5);
        let up: Vec<u8> = buf[..n].iter().map(|b| b.to_ascii_uppercase()).collect();
        self.inner.write_all(&up)?;
        self.written += n;
        Ok(n)
    }
    fn flush(&mut self) -> io::Result<()> { self.inner.flush() }
}

struct Failing { left: usize }
impl Read for Failing {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.left == 0 { return Err(io::Error::new(io::ErrorKind::ConnectionReset, "peer went away")); }
        let n = buf.len().min(self.left).min(3);
        for b in &mut buf[..n] { *b = b'z'; }
        self.left -= n;
        Ok(n)
    }
}

fn main() -> io::Result<()> {
    let text = "first line\nsecond line\r\n\nfourth,with,commas\nlast-no-newline";
    let cur = Cursor::new(text.as_bytes());
    let lines: Vec<String> = cur.lines().collect::<Result<_, _>>()?;
    println!("lines {:?}", lines);

    let mut cur = Cursor::new(text.as_bytes().to_vec());
    let mut line = String::new();
    let n = cur.read_line(&mut line)?;
    println!("read_line n={} {:?} pos={}", n, line, cur.position());
    let mut buf = Vec::new();
    let n = cur.read_until(b'\n', &mut buf)?;
    println!("read_until n={} {:?}", n, String::from_utf8_lossy(&buf));
    let mut exact = [0u8; 4];
    cur.read_exact(&mut exact)?;
    println!("read_exact {:?}", std::str::from_utf8(&exact).unwrap());
    cur.seek(SeekFrom::End(-4))?;
    let mut rest = String::new();
    cur.read_to_string(&mut rest)?;
    println!("seek end-4 -> {:?}", rest);
    let p = cur.seek(SeekFrom::Current(-10))?;
    println!("seek current -10 -> pos {}", p);
    cur.set_position(0);
    let parts: Vec<String> = cur.split(b',').map(|r| String::from_utf8(r.unwrap()).unwrap().replace('\n', "\\n")).collect();
    println!("split on ',' {:?}", parts);
    let err = Cursor::new(vec![1u8, 2]).read_exact(&mut [0u8; 3]).unwrap_err();
    println!("read_exact eof: kind={:?} msg={}", err.kind(), err);

    let chained = Cursor::new(b"abc".to_vec()).chain(Cursor::new(b"defgh".to_vec()));
    let mut taken = String::new();
    chained.take(6).read_to_string(&mut taken)?;
    println!("chain+take {:?}", taken);
    let bytes: Vec<u8> = Cursor::new(b"xyz").bytes().map(|b| b.unwrap() + 1).collect();
    println!("bytes {:?}", String::from_utf8(bytes).unwrap());

    let mut out: Vec<u8> = Vec::new();
    write!(out, "{}-{:03}", "num", 7)?;
    writeln!(out, " end")?;
    out.write_all(b"raw bytes\n")?;
    println!("vec writer {:?}", String::from_utf8(out.clone()).unwrap());

    let mut up = Upper { inner: Vec::new(), written: 0 };
    up.write_all(b"hello world, this is long")?;
    write!(up, " fmt {}", 42)?;
    println!("upper writer {:?} written={}", String::from_utf8(up.inner.clone()).unwrap(), up.written);

    let mut bw = BufWriter::with_capacity(8, Vec::new());
    for i in 0..5 { write!(bw, "[{}]", i)?; }
    println!("bufwriter buffered={} ", bw.buffer().len());
    let inner = bw.into_inner().map_err(|e| e.into_error())?;
    println!("bufwriter flushed {:?}", String::from_utf8(inner).unwrap());

    let mut src = Cursor::new("copy me please".as_bytes());
    let mut dst = Vec::new();
    let copied = io::copy(&mut src, &mut dst)?;
    println!("io::copy {} bytes -> {:?}", copied, String::from_utf8_lossy(&dst));

    let br = BufReader::with_capacity(4, Cursor::new("alpha beta\ngamma\n"));
    let words: Vec<String> = br.lines().flat_map(|l| l.unwrap().split(' ').map(String::from).collect::<Vec<_>>()).collect();
    println!("bufreader words {:?}", words);
    let mut br2 = BufReader::with_capacity(4, Cursor::new("0123456789"));
    let peek = br2.fill_buf()?.to_vec();
    br2.consume(2);
    let mut s = String::new();
    br2.read_to_string(&mut s)?;
    println!("fill_buf {:?} after consume {:?}", String::from_utf8(peek).unwrap(), s);

    let mut f = Failing { left: 7 };
    let mut got = Vec::new();
    let e = f.read_to_end(&mut got).unwrap_err();
    println!("failing reader got {} bytes, error kind={:?} msg={}", got.len(), e.kind(), e);
    let custom = io::Error::other("custom failure");
    println!("other: {:?} {}", custom.kind(), custom);
    let kinds = [io::ErrorKind::NotFound, io::ErrorKind::PermissionDenied, io::ErrorKind::InvalidData, io::ErrorKind::UnexpectedEof, io::ErrorKind::WouldBlock];
    println!("{:?}", kinds.iter().map(|k| io::Error::from(*k).to_string()).collect::<Vec<_>>());
    let wrapped: Result<(), io::Error> = Err(io::Error::new(io::ErrorKind::InvalidInput, format!("bad value {}", 5)));
    if let Err(e) = wrapped {
        println!("inner {:?} source? {}", e.get_ref().map(|x| x.to_string()), std::error::Error::source(&e).is_none());
    }
    let mut sink = io::sink();
    writeln!(sink, "discarded")?;
    let mut empty = String::new();
    io::empty().read_to_string(&mut empty)?;
    let rep: Vec<u8> = io::repeat(b'q').take(4).bytes().map(Result::unwrap).collect();
    println!("sink/empty/repeat ok {:?} {:?}", empty, rep);
    Ok(())
}
