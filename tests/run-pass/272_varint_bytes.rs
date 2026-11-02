// Binary encoding primitives (prost/bincode-like): LEB128 varints, zigzag, a byte Writer/Reader
// with Result errors, length-delimited strings, and a derive-free Encode/Decode trait pair.
use std::fmt;

#[derive(Debug, PartialEq)]
enum DecodeError {
    Eof { need: usize, at: usize },
    VarintTooLong,
    InvalidUtf8,
    BadTag(u8),
}

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            DecodeError::Eof { need, at } => write!(f, "unexpected eof: need {} bytes at {}", need, at),
            DecodeError::VarintTooLong => write!(f, "varint too long"),
            DecodeError::InvalidUtf8 => write!(f, "invalid utf-8"),
            DecodeError::BadTag(t) => write!(f, "bad tag {}", t),
        }
    }
}

#[derive(Default)]
struct Writer {
    buf: Vec<u8>,
}

impl Writer {
    fn varint(&mut self, mut v: u64) -> &mut Self {
        loop {
            let b = (v & 0x7f) as u8;
            v >>= 7;
            if v == 0 {
                self.buf.push(b);
                return self;
            }
            self.buf.push(b | 0x80);
        }
    }
    fn zigzag(&mut self, v: i64) -> &mut Self {
        self.varint(((v << 1) ^ (v >> 63)) as u64)
    }
    fn u32_le(&mut self, v: u32) -> &mut Self {
        self.buf.extend_from_slice(&v.to_le_bytes());
        self
    }
    fn f64_be(&mut self, v: f64) -> &mut Self {
        self.buf.extend_from_slice(&v.to_be_bytes());
        self
    }
    fn bytes(&mut self, b: &[u8]) -> &mut Self {
        self.varint(b.len() as u64);
        self.buf.extend_from_slice(b);
        self
    }
    fn str(&mut self, s: &str) -> &mut Self {
        self.bytes(s.as_bytes())
    }
}

struct Reader<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8], DecodeError> {
        if self.pos + n > self.buf.len() {
            return Err(DecodeError::Eof { need: n, at: self.pos });
        }
        let s = &self.buf[self.pos..self.pos + n];
        self.pos += n;
        Ok(s)
    }
    fn varint(&mut self) -> Result<u64, DecodeError> {
        let mut v = 0u64;
        for i in 0..10 {
            let b = self.take(1)?[0];
            v |= ((b & 0x7f) as u64) << (7 * i);
            if b & 0x80 == 0 {
                return Ok(v);
            }
        }
        Err(DecodeError::VarintTooLong)
    }
    fn zigzag(&mut self) -> Result<i64, DecodeError> {
        let u = self.varint()?;
        Ok((u >> 1) as i64 ^ -((u & 1) as i64))
    }
    fn u32_le(&mut self) -> Result<u32, DecodeError> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }
    fn f64_be(&mut self) -> Result<f64, DecodeError> {
        let arr: [u8; 8] = self.take(8)?.try_into().unwrap();
        Ok(f64::from_be_bytes(arr))
    }
    fn bytes(&mut self) -> Result<&'a [u8], DecodeError> {
        let n = self.varint()? as usize;
        self.take(n)
    }
    fn str(&mut self) -> Result<&'a str, DecodeError> {
        std::str::from_utf8(self.bytes()?).map_err(|_| DecodeError::InvalidUtf8)
    }
}

trait Codec: Sized {
    fn encode(&self, w: &mut Writer);
    fn decode(r: &mut Reader) -> Result<Self, DecodeError>;
}

#[derive(Debug, PartialEq, Clone)]
enum Shape {
    Circle { r: f64 },
    Poly(Vec<(i64, i64)>),
    Named(String, u32),
}

impl Codec for Shape {
    fn encode(&self, w: &mut Writer) {
        match self {
            Shape::Circle { r } => {
                w.varint(1).f64_be(*r);
            }
            Shape::Poly(pts) => {
                w.varint(2).varint(pts.len() as u64);
                for (x, y) in pts {
                    w.zigzag(*x).zigzag(*y);
                }
            }
            Shape::Named(n, id) => {
                w.varint(3).str(n).u32_le(*id);
            }
        }
    }
    fn decode(r: &mut Reader) -> Result<Self, DecodeError> {
        match r.varint()? {
            1 => Ok(Shape::Circle { r: r.f64_be()? }),
            2 => {
                let n = r.varint()?;
                let pts = (0..n).map(|_| Ok((r.zigzag()?, r.zigzag()?))).collect::<Result<Vec<_>, DecodeError>>()?;
                Ok(Shape::Poly(pts))
            }
            3 => Ok(Shape::Named(r.str()?.to_string(), r.u32_le()?)),
            t => Err(DecodeError::BadTag(t as u8)),
        }
    }
}

impl<T: Codec> Codec for Vec<T> {
    fn encode(&self, w: &mut Writer) {
        w.varint(self.len() as u64);
        for x in self {
            x.encode(w);
        }
    }
    fn decode(r: &mut Reader) -> Result<Self, DecodeError> {
        let n = r.varint()?;
        (0..n).map(|_| T::decode(r)).collect()
    }
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{:02x}", x)).collect::<Vec<_>>().join(" ")
}

fn main() {
    for v in [0u64, 1, 127, 128, 300, 16384, u32::MAX as u64, u64::MAX] {
        let mut w = Writer::default();
        w.varint(v);
        let back = Reader { buf: &w.buf, pos: 0 }.varint();
        println!("varint {:>20} -> [{}] -> {:?}", v, hex(&w.buf), back);
    }
    for v in [0i64, -1, 1, -64, 64, i64::MIN, i64::MAX] {
        let mut w = Writer::default();
        w.zigzag(v);
        println!("zigzag {:>20} -> [{}] -> {:?}", v, hex(&w.buf), Reader { buf: &w.buf, pos: 0 }.zigzag());
    }
    let shapes = vec![
        Shape::Circle { r: 1.5 },
        Shape::Poly(vec![(0, 0), (-3, 4), (100, -200)]),
        Shape::Named("héllo".into(), 0xDEADBEEF),
    ];
    let mut w = Writer::default();
    shapes.encode(&mut w);
    println!("encoded {} bytes: {}", w.buf.len(), hex(&w.buf));
    let decoded = Vec::<Shape>::decode(&mut Reader { buf: &w.buf, pos: 0 });
    println!("roundtrip equal: {}", decoded.as_ref() == Ok(&shapes));
    println!("{:?}", decoded.unwrap());

    for cut in [1, 5, 12, w.buf.len() - 1] {
        let r = Vec::<Shape>::decode(&mut Reader { buf: &w.buf[..cut], pos: 0 });
        println!("truncated to {:>2}: {}", cut, r.map(|v| format!("{} shapes", v.len())).unwrap_or_else(|e| e.to_string()));
    }
    let bad_tag = [1u8, 9];
    println!("{:?}", Vec::<Shape>::decode(&mut Reader { buf: &bad_tag, pos: 0 }));
    let bad_utf = [1u8, 3, 2, 0xff, 0xfe, 0, 0, 0, 0];
    println!("{:?}", Vec::<Shape>::decode(&mut Reader { buf: &bad_utf, pos: 0 }));
    let too_long = [0xffu8; 11];
    println!("{:?}", Reader { buf: &too_long, pos: 0 }.varint());
}
