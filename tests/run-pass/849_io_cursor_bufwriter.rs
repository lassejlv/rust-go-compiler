use std::io::{self, BufRead, BufWriter, Cursor, Read, Seek, SeekFrom, Write};
fn main() -> io::Result<()> {
    let stdout = io::stdout();
    let mut out = BufWriter::new(stdout.lock());
    writeln!(out, "buffered {}", 1)?;
    writeln!(out, "args {}", std::env::args().count())?;
    writeln!(out, "var {:?}", std::env::var("SURELY_NOT_SET_XYZ").is_err())?;
    let data = "alpha 1\nbeta 22\n\ngamma 333\r\ndelta";
    let cur = Cursor::new(data);
    for (i, l) in cur.lines().enumerate() { writeln!(out, "{} {:?}", i, l?)?; }
    let mut c2 = Cursor::new(data.as_bytes().to_vec());
    let mut buf = [0u8; 5];
    c2.read_exact(&mut buf)?;
    writeln!(out, "{:?} pos={}", std::str::from_utf8(&buf).unwrap(), c2.position())?;
    c2.seek(SeekFrom::End(-5))?;
    let mut rest = String::new();
    c2.read_to_string(&mut rest)?;
    writeln!(out, "{:?}", rest)?;
    c2.seek(SeekFrom::Start(6))?;
    let mut until = vec![];
    c2.read_until(b'\n', &mut until)?;
    writeln!(out, "{:?}", String::from_utf8_lossy(&until))?;
    let split: Vec<Vec<u8>> = Cursor::new("a,b,,c").split(b',').map(|r| r.unwrap()).collect();
    writeln!(out, "{:?}", split)?;
    let mut w = Cursor::new(Vec::new());
    write!(w, "{}-{}", 1, 2)?;
    w.write_all(b" tail")?;
    w.seek(SeekFrom::Start(0))?;
    w.write_all(b"X")?;
    writeln!(out, "{:?}", String::from_utf8(w.into_inner()).unwrap())?;
    let mut sink = io::sink();
    let copied = io::copy(&mut Cursor::new(vec![0u8; 1234]), &mut sink)?;
    writeln!(out, "copied {}", copied)?;
    let mut v: Vec<u8> = Vec::new();
    v.write_fmt(format_args!("{:>5}|", 42))?;
    writeln!(out, "{:?}", String::from_utf8(v).unwrap())?;
    let chained = Cursor::new("first ").chain(Cursor::new("second"));
    let mut s = String::new();
    chained.take(9).read_to_string(&mut s)?;
    writeln!(out, "{:?}", s)?;
    let e = io::Error::new(io::ErrorKind::InvalidData, "custom bad data");
    writeln!(out, "{} {:?}", e, e.kind())?;
    let e2: io::Error = io::ErrorKind::NotFound.into();
    writeln!(out, "{}", e2)?;
    out.flush()?;
    drop(out);
    print!("no newline at end");
    Ok(())
}
