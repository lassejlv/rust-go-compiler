use std::collections::BTreeMap;
use std::fmt::Write;

#[derive(Debug, Clone, PartialEq)]
enum Json {
    Null,
    Bool(bool),
    Num(f64),
    Str(String),
    Arr(Vec<Json>),
    Obj(BTreeMap<String, Json>),
}

trait Serialize {
    fn to_json(&self) -> Json;
}

trait Deserialize: Sized {
    fn from_json(j: &Json) -> Result<Self, String>;
}

impl Serialize for i64 {
    fn to_json(&self) -> Json {
        Json::Num(*self as f64)
    }
}
impl Serialize for f64 {
    fn to_json(&self) -> Json {
        Json::Num(*self)
    }
}
impl Serialize for bool {
    fn to_json(&self) -> Json {
        Json::Bool(*self)
    }
}
impl Serialize for String {
    fn to_json(&self) -> Json {
        Json::Str(self.clone())
    }
}
impl<T: Serialize> Serialize for Vec<T> {
    fn to_json(&self) -> Json {
        Json::Arr(self.iter().map(|x| x.to_json()).collect())
    }
}
impl<T: Serialize> Serialize for Option<T> {
    fn to_json(&self) -> Json {
        match self {
            Some(x) => x.to_json(),
            None => Json::Null,
        }
    }
}

impl Deserialize for i64 {
    fn from_json(j: &Json) -> Result<Self, String> {
        match j {
            Json::Num(n) if n.fract() == 0.0 => Ok(*n as i64),
            other => Err(format!("expected integer, got {:?}", other)),
        }
    }
}
impl Deserialize for bool {
    fn from_json(j: &Json) -> Result<Self, String> {
        if let Json::Bool(b) = j { Ok(*b) } else { Err(format!("expected bool, got {:?}", j)) }
    }
}
impl Deserialize for String {
    fn from_json(j: &Json) -> Result<Self, String> {
        if let Json::Str(s) = j { Ok(s.clone()) } else { Err(format!("expected string, got {:?}", j)) }
    }
}
impl<T: Deserialize> Deserialize for Vec<T> {
    fn from_json(j: &Json) -> Result<Self, String> {
        match j {
            Json::Arr(items) => items.iter().map(T::from_json).collect(),
            other => Err(format!("expected array, got {:?}", other)),
        }
    }
}
impl<T: Deserialize> Deserialize for Option<T> {
    fn from_json(j: &Json) -> Result<Self, String> {
        match j {
            Json::Null => Ok(None),
            other => T::from_json(other).map(Some),
        }
    }
}

fn field<T: Deserialize>(obj: &BTreeMap<String, Json>, name: &str) -> Result<T, String> {
    let v = obj.get(name).unwrap_or(&Json::Null);
    T::from_json(v).map_err(|e| format!("field '{}': {}", name, e))
}

#[derive(Debug, PartialEq)]
struct User {
    id: i64,
    name: String,
    admin: bool,
    tags: Vec<String>,
    manager: Option<i64>,
}

impl Serialize for User {
    fn to_json(&self) -> Json {
        let mut m = BTreeMap::new();
        m.insert("id".to_string(), self.id.to_json());
        m.insert("name".to_string(), self.name.to_json());
        m.insert("admin".to_string(), self.admin.to_json());
        m.insert("tags".to_string(), self.tags.to_json());
        m.insert("manager".to_string(), self.manager.to_json());
        Json::Obj(m)
    }
}

impl Deserialize for User {
    fn from_json(j: &Json) -> Result<Self, String> {
        let Json::Obj(o) = j else {
            return Err("expected object".to_string());
        };
        Ok(User { id: field(o, "id")?, name: field(o, "name")?, admin: field(o, "admin")?, tags: field(o, "tags")?, manager: field(o, "manager")? })
    }
}

fn write_json(j: &Json, out: &mut String) {
    match j {
        Json::Null => out.push_str("null"),
        Json::Bool(b) => write!(out, "{}", b).unwrap(),
        Json::Num(n) => write!(out, "{}", n).unwrap(),
        Json::Str(s) => {
            out.push('"');
            for c in s.chars() {
                match c {
                    '"' => out.push_str("\\\""),
                    '\\' => out.push_str("\\\\"),
                    '\n' => out.push_str("\\n"),
                    c => out.push(c),
                }
            }
            out.push('"');
        }
        Json::Arr(a) => {
            out.push('[');
            for (i, x) in a.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_json(x, out);
            }
            out.push(']');
        }
        Json::Obj(o) => {
            out.push('{');
            for (i, (k, v)) in o.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_json(&Json::Str(k.clone()), out);
                out.push(':');
                write_json(v, out);
            }
            out.push('}');
        }
    }
}

fn parse(s: &str) -> Result<Json, String> {
    fn ws(b: &[u8], i: &mut usize) {
        while *i < b.len() && (b[*i] as char).is_whitespace() {
            *i += 1;
        }
    }
    fn val(b: &[u8], i: &mut usize) -> Result<Json, String> {
        ws(b, i);
        match b.get(*i) {
            None => Err("unexpected end".into()),
            Some(b'n') if b[*i..].starts_with(b"null") => { *i += 4; Ok(Json::Null) }
            Some(b't') if b[*i..].starts_with(b"true") => { *i += 4; Ok(Json::Bool(true)) }
            Some(b'f') if b[*i..].starts_with(b"false") => { *i += 5; Ok(Json::Bool(false)) }
            Some(b'"') => {
                *i += 1;
                let mut s = String::new();
                while b[*i] != b'"' {
                    if b[*i] == b'\\' {
                        *i += 1;
                        s.push(match b[*i] { b'n' => '\n', c => c as char });
                    } else {
                        s.push(b[*i] as char);
                    }
                    *i += 1;
                }
                *i += 1;
                Ok(Json::Str(s))
            }
            Some(b'[') => {
                *i += 1;
                let mut v = Vec::new();
                loop {
                    ws(b, i);
                    if b[*i] == b']' { *i += 1; break; }
                    v.push(val(b, i)?);
                    ws(b, i);
                    if b[*i] == b',' { *i += 1; }
                }
                Ok(Json::Arr(v))
            }
            Some(b'{') => {
                *i += 1;
                let mut m = BTreeMap::new();
                loop {
                    ws(b, i);
                    if b[*i] == b'}' { *i += 1; break; }
                    let Json::Str(k) = val(b, i)? else { return Err("key must be string".into()) };
                    ws(b, i);
                    if b[*i] != b':' { return Err(format!("expected ':' at {}", i)); }
                    *i += 1;
                    m.insert(k, val(b, i)?);
                    ws(b, i);
                    if b[*i] == b',' { *i += 1; }
                }
                Ok(Json::Obj(m))
            }
            Some(_) => {
                let st = *i;
                while *i < b.len() && (b[*i] == b'-' || b[*i] == b'.' || b[*i].is_ascii_digit() || b[*i] == b'e') {
                    *i += 1;
                }
                std::str::from_utf8(&b[st..*i]).unwrap().parse().map(Json::Num).map_err(|_| format!("bad token at {}", st))
            }
        }
    }
    let mut i = 0;
    val(s.as_bytes(), &mut i)
}

fn main() {
    let users = vec![
        User { id: 1, name: "Ann \"the boss\"".into(), admin: true, tags: vec!["ops".into(), "dev".into()], manager: None },
        User { id: 2, name: "Bob\nNewline".into(), admin: false, tags: vec![], manager: Some(1) },
    ];
    let j = users.to_json();
    let mut s = String::new();
    write_json(&j, &mut s);
    println!("{}", s);
    let parsed = parse(&s).unwrap();
    println!("roundtrip json equal: {}", parsed == j);
    let back: Vec<User> = Vec::from_json(&parsed).unwrap();
    println!("roundtrip users equal: {}", back == users);
    println!("{:?}", back[1]);

    let bad_inputs = [
        r#"{"id": 3, "name": "C", "admin": "yes", "tags": []}"#,
        r#"{"id": 4.5, "name": "D", "admin": false, "tags": []}"#,
        r#"{"id": 5, "name": "E", "admin": false, "tags": ["x", 7]}"#,
        r#"{"id": 6, "name": "F", "admin": false, "tags": ["x"], "manager": 2}"#,
        r#"[1, 2]"#,
    ];
    for inp in bad_inputs {
        let r = parse(inp).and_then(|j| User::from_json(&j));
        match r {
            Ok(u) => println!("ok {:?}", u),
            Err(e) => println!("error: {}", e),
        }
    }
    let nums: Vec<Option<i64>> = Vec::from_json(&parse("[1, null, -3]").unwrap()).unwrap();
    println!("{:?}", nums);
    let mut out = String::new();
    write_json(&vec![1.5f64, -0.25, 1e21].to_json(), &mut out);
    println!("{}", out);
}
