// A protobuf-like binary wire format over std::io::{Read, Write}: varints,
// zigzag, fixed32/64, length-delimited fields, an Encode/Decode trait pair with
// impls for primitives, Vec, Option, String, nested messages, unknown-field
// skipping, io::Error kinds, Cursor/BufReader/BufWriter, and hex dumps.
use std::collections::BTreeMap;
use std::fmt::Write as FmtWrite;
use std::io::{self, BufRead, BufReader, BufWriter, Cursor, ErrorKind, Read, Write};

fn write_varint<W: Write + ?Sized>(w: &mut W, mut v: u64) -> io::Result<usize> {
    let mut buf = [0u8; 10];
    let mut n = 0;
    loop {
        let b = (v & 0x7f) as u8;
        v >>= 7;
        if v == 0 {
            buf[n] = b;
            n += 1;
            break;
        }
        buf[n] = b | 0x80;
        n += 1;
    }
    w.write_all(&buf[..n])?;
    Ok(n)
}

fn read_varint<R: Read + ?Sized>(r: &mut R) -> io::Result<u64> {
    let mut v = 0u64;
    for i in 0..10 {
        let mut b = [0u8; 1];
        r.read_exact(&mut b)?;
        v |= ((b[0] & 0x7f) as u64) << (7 * i);
        if b[0] & 0x80 == 0 {
            return Ok(v);
        }
    }
    Err(io::Error::new(ErrorKind::InvalidData, "varint too long"))
}

fn zigzag(v: i64) -> u64 {
    ((v << 1) ^ (v >> 63)) as u64
}
fn unzigzag(v: u64) -> i64 {
    ((v >> 1) as i64) ^ -((v & 1) as i64)
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum WireType {
    Varint = 0,
    Fixed64 = 1,
    Len = 2,
    Fixed32 = 5,
}

impl TryFrom<u64> for WireType {
    type Error = io::Error;
    fn try_from(v: u64) -> io::Result<Self> {
        Ok(match v {
            0 => WireType::Varint,
            1 => WireType::Fixed64,
            2 => WireType::Len,
            5 => WireType::Fixed32,
            x => return Err(io::Error::new(ErrorKind::InvalidData, format!("bad wire type {}", x))),
        })
    }
}

trait Encode {
    const WIRE: WireType;
    fn encode_raw(&self, w: &mut dyn Write) -> io::Result<()>;
    fn encode_field(&self, field: u32, w: &mut dyn Write) -> io::Result<()> {
        write_varint(w, (field as u64) << 3 | Self::WIRE as u64)?;
        self.encode_raw(w)
    }
}

trait Decode: Sized {
    fn decode_raw(r: &mut dyn Read, wt: WireType) -> io::Result<Self>;
}

impl Encode for u64 {
    const WIRE: WireType = WireType::Varint;
    fn encode_raw(&self, w: &mut dyn Write) -> io::Result<()> {
        write_varint(w, *self).map(|_| ())
    }
}
impl Decode for u64 {
    fn decode_raw(r: &mut dyn Read, _: WireType) -> io::Result<Self> {
        read_varint(r)
    }
}
impl Encode for i64 {
    const WIRE: WireType = WireType::Varint;
    fn encode_raw(&self, w: &mut dyn Write) -> io::Result<()> {
        write_varint(w, zigzag(*self)).map(drop)
    }
}
impl Decode for i64 {
    fn decode_raw(r: &mut dyn Read, _: WireType) -> io::Result<Self> {
        read_varint(r).map(unzigzag)
    }
}
impl Encode for bool {
    const WIRE: WireType = WireType::Varint;
    fn encode_raw(&self, w: &mut dyn Write) -> io::Result<()> {
        w.write_all(&[*self as u8])
    }
}
impl Decode for bool {
    fn decode_raw(r: &mut dyn Read, _: WireType) -> io::Result<Self> {
        Ok(read_varint(r)? != 0)
    }
}
impl Encode for f64 {
    const WIRE: WireType = WireType::Fixed64;
    fn encode_raw(&self, w: &mut dyn Write) -> io::Result<()> {
        w.write_all(&self.to_le_bytes())
    }
}
impl Decode for f64 {
    fn decode_raw(r: &mut dyn Read, _: WireType) -> io::Result<Self> {
        let mut b = [0u8; 8];
        r.read_exact(&mut b)?;
        Ok(f64::from_le_bytes(b))
    }
}
impl Encode for f32 {
    const WIRE: WireType = WireType::Fixed32;
    fn encode_raw(&self, w: &mut dyn Write) -> io::Result<()> {
        w.write_all(&self.to_bits().to_le_bytes())
    }
}
impl Decode for f32 {
    fn decode_raw(r: &mut dyn Read, _: WireType) -> io::Result<Self> {
        let mut b = [0u8; 4];
        r.read_exact(&mut b)?;
        Ok(f32::from_bits(u32::from_le_bytes(b)))
    }
}
impl Encode for String {
    const WIRE: WireType = WireType::Len;
    fn encode_raw(&self, w: &mut dyn Write) -> io::Result<()> {
        write_varint(w, self.len() as u64)?;
        w.write_all(self.as_bytes())
    }
}
impl Decode for String {
    fn decode_raw(r: &mut dyn Read, _: WireType) -> io::Result<Self> {
        let n = read_varint(r)? as usize;
        let mut buf = vec![0u8; n];
        r.read_exact(&mut buf)?;
        String::from_utf8(buf).map_err(|e| io::Error::new(ErrorKind::InvalidData, e))
    }
}
impl Encode for Vec<u8> {
    const WIRE: WireType = WireType::Len;
    fn encode_raw(&self, w: &mut dyn Write) -> io::Result<()> {
        write_varint(w, self.len() as u64)?;
        w.write_all(self)
    }
}
impl Decode for Vec<u8> {
    fn decode_raw(r: &mut dyn Read, _: WireType) -> io::Result<Self> {
        let n = read_varint(r)? as usize;
        let mut buf = Vec::new();
        r.take(n as u64).read_to_end(&mut buf)?;
        if buf.len() != n {
            return Err(io::Error::from(ErrorKind::UnexpectedEof));
        }
        Ok(buf)
    }
}

/// A message type: encodes as length-delimited when nested.
trait Message: Default {
    fn write_fields(&self, w: &mut dyn Write) -> io::Result<()>;
    fn merge_field(&mut self, field: u32, wt: WireType, r: &mut dyn Read) -> io::Result<bool>;

    fn to_bytes(&self) -> Vec<u8> {
        let mut v = Vec::new();
        self.write_fields(&mut v).expect("vec write");
        v
    }
    fn from_bytes(b: &[u8]) -> io::Result<Self> {
        let mut msg = Self::default();
        let mut cur = Cursor::new(b);
        while (cur.position() as usize) < b.len() {
            let key = read_varint(&mut cur)?;
            let wt = WireType::try_from(key & 7)?;
            let field = (key >> 3) as u32;
            if !msg.merge_field(field, wt, &mut cur)? {
                skip(&mut cur, wt)?;
            }
        }
        Ok(msg)
    }
}

fn skip(r: &mut dyn Read, wt: WireType) -> io::Result<()> {
    let n = match wt {
        WireType::Varint => return read_varint(r).map(|_| ()),
        WireType::Fixed64 => 8,
        WireType::Fixed32 => 4,
        WireType::Len => read_varint(r)?,
    };
    let copied = io::copy(&mut r.take(n), &mut io::sink())?;
    if copied != n {
        return Err(io::Error::new(ErrorKind::UnexpectedEof, "short skip"));
    }
    Ok(())
}

fn write_msg<M: Message>(field: u32, m: &M, w: &mut dyn Write) -> io::Result<()> {
    let body = m.to_bytes();
    write_varint(w, (field as u64) << 3 | 2)?;
    write_varint(w, body.len() as u64)?;
    w.write_all(&body)
}

fn read_msg<M: Message>(r: &mut dyn Read) -> io::Result<M> {
    let bytes = Vec::<u8>::decode_raw(r, WireType::Len)?;
    M::from_bytes(&bytes)
}

#[derive(Debug, Default, Clone, PartialEq)]
struct Point {
    x: i64,
    y: i64,
    label: Option<String>,
}

impl Message for Point {
    fn write_fields(&self, w: &mut dyn Write) -> io::Result<()> {
        self.x.encode_field(1, w)?;
        self.y.encode_field(2, w)?;
        if let Some(l) = &self.label {
            l.encode_field(3, w)?;
        }
        Ok(())
    }
    fn merge_field(&mut self, f: u32, wt: WireType, r: &mut dyn Read) -> io::Result<bool> {
        match f {
            1 => self.x = i64::decode_raw(r, wt)?,
            2 => self.y = i64::decode_raw(r, wt)?,
            3 => self.label = Some(String::decode_raw(r, wt)?),
            _ => return Ok(false),
        }
        Ok(true)
    }
}

#[derive(Debug, Default, Clone, PartialEq)]
struct Shape {
    name: String,
    points: Vec<Point>,
    closed: bool,
    area: f64,
    weight: f32,
    tags: Vec<String>,
    blob: Vec<u8>,
    id: u64,
}

impl Message for Shape {
    fn write_fields(&self, w: &mut dyn Write) -> io::Result<()> {
        self.name.encode_field(1, w)?;
        for p in &self.points {
            write_msg(2, p, w)?;
        }
        if self.closed {
            self.closed.encode_field(3, w)?;
        }
        self.area.encode_field(4, w)?;
        self.weight.encode_field(5, w)?;
        for t in &self.tags {
            t.encode_field(6, w)?;
        }
        if !self.blob.is_empty() {
            self.blob.encode_field(7, w)?;
        }
        self.id.encode_field(8, w)
    }
    fn merge_field(&mut self, f: u32, wt: WireType, r: &mut dyn Read) -> io::Result<bool> {
        match f {
            1 => self.name = String::decode_raw(r, wt)?,
            2 => self.points.push(read_msg(r)?),
            3 => self.closed = bool::decode_raw(r, wt)?,
            4 => self.area = f64::decode_raw(r, wt)?,
            5 => self.weight = f32::decode_raw(r, wt)?,
            6 => self.tags.push(String::decode_raw(r, wt)?),
            7 => self.blob = Vec::<u8>::decode_raw(r, wt)?,
            8 => self.id = u64::decode_raw(r, wt)?,
            _ => return Ok(false),
        }
        Ok(true)
    }
}

fn hexdump(b: &[u8]) -> String {
    let mut s = String::new();
    for (i, chunk) in b.chunks(16).enumerate() {
        write!(s, "{:04x}: ", i * 16).unwrap();
        for byte in chunk {
            write!(s, "{:02x} ", byte).unwrap();
        }
        for _ in chunk.len()..16 {
            s.push_str("   ");
        }
        let ascii: String = chunk.iter().map(|&c| if c.is_ascii_graphic() { c as char } else { '.' }).collect();
        writeln!(s, "|{}|", ascii).unwrap();
    }
    s
}

/// Records framed as varint length + payload, written through a BufWriter
fn write_records<W: Write>(w: W, shapes: &[Shape]) -> io::Result<W> {
    let mut bw = BufWriter::with_capacity(16, w);
    for s in shapes {
        let b = s.to_bytes();
        write_varint(&mut bw, b.len() as u64)?;
        bw.write_all(&b)?;
    }
    bw.flush()?;
    bw.into_inner().map_err(|e| e.into_error())
}

fn read_records<R: Read>(r: R) -> io::Result<Vec<Shape>> {
    let mut br = BufReader::with_capacity(8, r);
    let mut out = Vec::new();
    loop {
        if br.fill_buf()?.is_empty() {
            return Ok(out);
        }
        let n = read_varint(&mut br)? as usize;
        let mut b = vec![0; n];
        br.read_exact(&mut b)?;
        out.push(Shape::from_bytes(&b)?);
    }
}

fn main() -> io::Result<()> {
    let mut v = Vec::new();
    for x in [0u64, 1, 127, 128, 300, 16384, u64::MAX] {
        let n = write_varint(&mut v, x)?;
        print!("{}:{} ", x, n);
    }
    println!();
    let mut cur = Cursor::new(&v);
    let back: Vec<u64> = (0..7).map(|_| read_varint(&mut cur)).collect::<io::Result<_>>()?;
    println!("{:?}", back);
    println!("{:?}", [0i64, -1, 1, -2, i64::MAX, i64::MIN].iter().map(|&x| (zigzag(x), unzigzag(zigzag(x)) == x)).collect::<Vec<_>>());

    let shape = Shape {
        name: "triangle ▲".into(),
        points: vec![Point { x: 0, y: 0, label: Some("origin".into()) }, Point { x: 10, y: -5, label: None }, Point { x: -3, y: 7, label: Some(String::new()) }],
        closed: true,
        area: 47.5,
        weight: 0.1,
        tags: vec!["geo".into(), "test".into()],
        blob: vec![0, 255, 7],
        id: 1 << 40,
    };
    let bytes = shape.to_bytes();
    print!("{}", hexdump(&bytes));
    let decoded = Shape::from_bytes(&bytes)?;
    println!("roundtrip equal: {}", decoded == shape);
    println!("{:?}", decoded.points[2]);

    // forward compatibility: unknown fields are skipped
    let mut extended = bytes.clone();
    99u64.encode_field(15, &mut extended)?;
    "future".to_string().encode_field(16, &mut extended)?;
    1.5f64.encode_field(17, &mut extended)?;
    2.5f32.encode_field(18, &mut extended)?;
    let d2 = Shape::from_bytes(&extended)?;
    println!("with unknown fields: equal={} extra bytes={}", d2 == shape, extended.len() - bytes.len());

    // corrupted inputs produce io errors with kinds
    let cases: Vec<(&str, Vec<u8>)> = vec![
        ("truncated", bytes[..bytes.len() - 3].to_vec()),
        ("bad wiretype", vec![0x0b, 0x01]),
        ("long varint", vec![0x08, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x01]),
        ("bad utf8", vec![0x0a, 0x02, 0xc3, 0x28]),
        ("short blob", vec![0x3a, 0x05, 1, 2]),
    ];
    for (name, b) in cases {
        match Shape::from_bytes(&b) {
            Ok(s) => println!("{}: ok?? {:?}", name, s.name),
            Err(e) => println!("{}: {:?} - {}", name, e.kind(), e),
        }
    }

    // framed records through buffered I/O
    let shapes: Vec<Shape> = (0..5u64)
        .map(|i| Shape { name: format!("s{}", i), points: (0..i as i64).map(|k| Point { x: k, y: -k * k, label: None }).collect(), id: i * 1000, area: i as f64 / 3.0, ..Default::default() })
        .collect();
    let buf = write_records(Vec::new(), &shapes)?;
    println!("records: {} bytes", buf.len());
    let back = read_records(Cursor::new(buf.clone()))?;
    println!("records equal: {} ids {:?}", back == shapes, back.iter().map(|s| s.id).collect::<Vec<_>>());
    let trunc = read_records(&buf[..buf.len() - 1]);
    println!("truncated stream: {:?}", trunc.map(|v| v.len()).map_err(|e| e.kind()));

    // text side: BufRead lines and split
    let text = "alpha,1\nbeta,22\r\ngamma,333\n\ndelta,x\n";
    let mut sums = BTreeMap::new();
    for (i, line) in Cursor::new(text).lines().enumerate() {
        let line = line?;
        let line = line.trim_end_matches('\r');
        if line.is_empty() {
            continue;
        }
        let (k, v) = line.split_once(',').unwrap();
        match v.parse::<u32>() {
            Ok(n) => {
                sums.insert(k.to_string(), n);
            }
            Err(e) => println!("line {}: {}", i + 1, e),
        }
    }
    println!("{:?}", sums);
    let parts: Vec<String> = Cursor::new(b"a;bb;;ccc".to_vec()).split(b';').map(|p| String::from_utf8(p.unwrap()).unwrap()).collect();
    println!("{:?}", parts);
    let mut chained = Cursor::new(b"hello ".to_vec()).chain(Cursor::new(b"world".to_vec()));
    let mut s = String::new();
    chained.read_to_string(&mut s)?;
    let mut out = Vec::new();
    writeln!(out, "{} ({} bytes)", s, s.len())?;
    write!(&mut out, "{:>5}|{:<5}|", 1, 2)?;
    io::stdout().write_all(&out)?;
    println!();
    let e = io::Error::new(ErrorKind::Other, "custom failure");
    let e2: io::Error = ErrorKind::NotFound.into();
    println!("{} / {:?} / {}", e, e2.kind(), e2);
    Ok(())
}
