use std::collections::BTreeMap;
use std::fmt;

// ---- Serialization side ----
pub trait Serializer: Sized {
    type Ok;
    type Error: fmt::Debug;
    type SerializeSeq: SerializeSeq<Ok = Self::Ok, Error = Self::Error>;
    type SerializeMap: SerializeMap<Ok = Self::Ok, Error = Self::Error>;
    fn serialize_i64(self, v: i64) -> Result<Self::Ok, Self::Error>;
    fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error>;
    fn serialize_bool(self, v: bool) -> Result<Self::Ok, Self::Error>;
    fn serialize_none(self) -> Result<Self::Ok, Self::Error>;
    fn serialize_some<T: ?Sized + Serialize>(self, v: &T) -> Result<Self::Ok, Self::Error>;
    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error>;
    fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeMap, Self::Error>;
}
pub trait SerializeSeq {
    type Ok;
    type Error;
    fn serialize_element<T: ?Sized + Serialize>(&mut self, v: &T) -> Result<(), Self::Error>;
    fn end(self) -> Result<Self::Ok, Self::Error>;
}
pub trait SerializeMap {
    type Ok;
    type Error;
    fn serialize_entry<K: ?Sized + Serialize, V: ?Sized + Serialize>(&mut self, k: &K, v: &V) -> Result<(), Self::Error>;
    fn end(self) -> Result<Self::Ok, Self::Error>;
}
pub trait Serialize {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error>;
}

impl Serialize for i64 { fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> { s.serialize_i64(*self) } }
impl Serialize for i32 { fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> { s.serialize_i64(*self as i64) } }
impl Serialize for bool { fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> { s.serialize_bool(*self) } }
impl Serialize for str { fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> { s.serialize_str(self) } }
impl Serialize for String { fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> { s.serialize_str(self) } }
impl<T: Serialize + ?Sized> Serialize for &T { fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> { (**self).serialize(s) } }
impl<T: Serialize> Serialize for Option<T> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        match self { Some(v) => s.serialize_some(v), None => s.serialize_none() }
    }
}
impl<T: Serialize> Serialize for [T] {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut seq = s.serialize_seq(Some(self.len()))?;
        for x in self { seq.serialize_element(x)?; }
        seq.end()
    }
}
impl<T: Serialize> Serialize for Vec<T> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> { self.as_slice().serialize(s) }
}
impl<K: Serialize, V: Serialize> Serialize for BTreeMap<K, V> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut m = s.serialize_map(Some(self.len()))?;
        for (k, v) in self { m.serialize_entry(k, v)?; }
        m.end()
    }
}
impl<A: Serialize, B: Serialize> Serialize for (A, B) {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut seq = s.serialize_seq(Some(2))?;
        seq.serialize_element(&self.0)?;
        seq.serialize_element(&self.1)?;
        seq.end()
    }
}

// JSON writer serializer
pub struct Json<'a> { out: &'a mut String }
pub struct JsonSeq<'a> { out: &'a mut String, first: bool }
#[derive(Debug)]
pub struct JsonErr(String);

impl<'a> Serializer for Json<'a> {
    type Ok = ();
    type Error = JsonErr;
    type SerializeSeq = JsonSeq<'a>;
    type SerializeMap = JsonSeq<'a>;
    fn serialize_i64(self, v: i64) -> Result<(), JsonErr> { self.out.push_str(&v.to_string()); Ok(()) }
    fn serialize_str(self, v: &str) -> Result<(), JsonErr> {
        if v.contains('"') { return Err(JsonErr(format!("quote in {:?}", v))); }
        self.out.push('"'); self.out.push_str(v); self.out.push('"'); Ok(())
    }
    fn serialize_bool(self, v: bool) -> Result<(), JsonErr> { self.out.push_str(if v { "true" } else { "false" }); Ok(()) }
    fn serialize_none(self) -> Result<(), JsonErr> { self.out.push_str("null"); Ok(()) }
    fn serialize_some<T: ?Sized + Serialize>(self, v: &T) -> Result<(), JsonErr> { v.serialize(self) }
    fn serialize_seq(self, _l: Option<usize>) -> Result<JsonSeq<'a>, JsonErr> { self.out.push('['); Ok(JsonSeq { out: self.out, first: true }) }
    fn serialize_map(self, _l: Option<usize>) -> Result<JsonSeq<'a>, JsonErr> { self.out.push('{'); Ok(JsonSeq { out: self.out, first: true }) }
}
impl<'a> SerializeSeq for JsonSeq<'a> {
    type Ok = ();
    type Error = JsonErr;
    fn serialize_element<T: ?Sized + Serialize>(&mut self, v: &T) -> Result<(), JsonErr> {
        if !self.first { self.out.push(','); }
        self.first = false;
        v.serialize(Json { out: &mut *self.out })
    }
    fn end(self) -> Result<(), JsonErr> { self.out.push(']'); Ok(()) }
}
impl<'a> SerializeMap for JsonSeq<'a> {
    type Ok = ();
    type Error = JsonErr;
    fn serialize_entry<K: ?Sized + Serialize, V: ?Sized + Serialize>(&mut self, k: &K, v: &V) -> Result<(), JsonErr> {
        if !self.first { self.out.push(','); }
        self.first = false;
        k.serialize(Json { out: &mut *self.out })?;
        self.out.push(':');
        v.serialize(Json { out: &mut *self.out })
    }
    fn end(self) -> Result<(), JsonErr> { self.out.push('}'); Ok(()) }
}

// A counting serializer with a different Ok type
pub struct Count;
pub struct CountSeq(usize);
impl Serializer for Count {
    type Ok = usize;
    type Error = std::convert::Infallible;
    type SerializeSeq = CountSeq;
    type SerializeMap = CountSeq;
    fn serialize_i64(self, _: i64) -> Result<usize, Self::Error> { Ok(1) }
    fn serialize_str(self, _: &str) -> Result<usize, Self::Error> { Ok(1) }
    fn serialize_bool(self, _: bool) -> Result<usize, Self::Error> { Ok(1) }
    fn serialize_none(self) -> Result<usize, Self::Error> { Ok(0) }
    fn serialize_some<T: ?Sized + Serialize>(self, v: &T) -> Result<usize, Self::Error> { v.serialize(self) }
    fn serialize_seq(self, _: Option<usize>) -> Result<CountSeq, Self::Error> { Ok(CountSeq(0)) }
    fn serialize_map(self, _: Option<usize>) -> Result<CountSeq, Self::Error> { Ok(CountSeq(0)) }
}
impl SerializeSeq for CountSeq {
    type Ok = usize;
    type Error = std::convert::Infallible;
    fn serialize_element<T: ?Sized + Serialize>(&mut self, v: &T) -> Result<(), Self::Error> { self.0 += v.serialize(Count)?; Ok(()) }
    fn end(self) -> Result<usize, Self::Error> { Ok(self.0) }
}
impl SerializeMap for CountSeq {
    type Ok = usize;
    type Error = std::convert::Infallible;
    fn serialize_entry<K: ?Sized + Serialize, V: ?Sized + Serialize>(&mut self, k: &K, v: &V) -> Result<(), Self::Error> {
        self.0 += k.serialize(Count)? + v.serialize(Count)?;
        Ok(())
    }
    fn end(self) -> Result<usize, Self::Error> { Ok(self.0) }
}

struct User { name: String, age: i32, tags: Vec<&'static str>, boss: Option<Box<User>> }
impl Serialize for User {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut m = s.serialize_map(Some(4))?;
        m.serialize_entry("name", &self.name)?;
        m.serialize_entry("age", &self.age)?;
        m.serialize_entry("tags", &self.tags)?;
        m.serialize_entry("boss", &self.boss.as_deref())?;
        m.end()
    }
}

fn to_json<T: Serialize + ?Sized>(v: &T) -> Result<String, JsonErr> {
    let mut s = String::new();
    v.serialize(Json { out: &mut s })?;
    Ok(s)
}

// ---- Deserialization side (visitor) ----
pub trait Visitor<'de>: Sized {
    type Value;
    fn expecting(&self) -> &'static str;
    fn visit_i64(self, v: i64) -> Result<Self::Value, String> { let _ = v; Err(format!("unexpected int, expected {}", self.expecting())) }
    fn visit_str(self, v: &'de str) -> Result<Self::Value, String> { let _ = v; Err(format!("unexpected str, expected {}", self.expecting())) }
}
pub trait Deserializer<'de> {
    fn deserialize_any<V: Visitor<'de>>(self, v: V) -> Result<V::Value, String>;
}
pub trait Deserialize<'de>: Sized {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, String>;
}
struct Token<'de>(&'de str);
impl<'de> Deserializer<'de> for Token<'de> {
    fn deserialize_any<V: Visitor<'de>>(self, v: V) -> Result<V::Value, String> {
        match self.0.parse::<i64>() {
            Ok(n) => v.visit_i64(n),
            Err(_) => v.visit_str(self.0),
        }
    }
}
impl<'de> Deserialize<'de> for i64 {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<i64, String> {
        struct V;
        impl<'de> Visitor<'de> for V {
            type Value = i64;
            fn expecting(&self) -> &'static str { "an integer" }
            fn visit_i64(self, v: i64) -> Result<i64, String> { Ok(v) }
        }
        d.deserialize_any(V)
    }
}
impl<'de: 'a, 'a> Deserialize<'de> for &'a str {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<&'a str, String> {
        struct V;
        impl<'de> Visitor<'de> for V {
            type Value = &'de str;
            fn expecting(&self) -> &'static str { "a borrowed string" }
            fn visit_str(self, v: &'de str) -> Result<&'de str, String> { Ok(v) }
        }
        d.deserialize_any(V)
    }
}
fn from_tokens<'de, T: Deserialize<'de>>(src: &'de str) -> Vec<Result<T, String>> {
    src.split_whitespace().map(|t| T::deserialize(Token(t))).collect()
}

fn main() {
    let boss = User { name: "Ann".into(), age: 50, tags: vec![], boss: None };
    let u = User { name: "Bob".into(), age: 30, tags: vec!["a", "b"], boss: Some(Box::new(boss)) };
    println!("{:?}", to_json(&u));
    println!("{:?}", u.serialize(Count));
    let mut m = BTreeMap::new();
    m.insert("k1".to_string(), vec![Some(1i64), None]);
    m.insert("k2".to_string(), vec![]);
    println!("{:?}", to_json(&m));
    println!("{:?}", to_json(&vec![(true, "x"), (false, "y")]));
    println!("{:?}", to_json("bad\"quote"));
    println!("{:?}", to_json(&[1i32, 2, 3][..]));
    println!("{:?}", m.serialize(Count));
    let src = String::from("12 abc -4");
    println!("{:?}", from_tokens::<i64>(&src));
    println!("{:?}", from_tokens::<&str>(&src));
}
