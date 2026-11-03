// A tiny template engine: lexer for {{ var }}, {% if %}/{% else %}/{% endif %},
// {% for x in list %}/{% endfor %}, filters (upper, len, default), nested
// contexts with scoped variables, and error reporting with positions.
use std::collections::BTreeMap;
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
enum Val { Str(String), Num(i64), Bool(bool), List(Vec<Val>), Map(BTreeMap<String, Val>) }

impl fmt::Display for Val {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Val::Str(s) => write!(f, "{}", s),
            Val::Num(n) => write!(f, "{}", n),
            Val::Bool(b) => write!(f, "{}", b),
            Val::List(l) => write!(f, "{}", l.iter().map(|v| v.to_string()).collect::<Vec<_>>().join(", ")),
            Val::Map(m) => write!(f, "{{{}}}", m.keys().cloned().collect::<Vec<_>>().join(",")),
        }
    }
}
impl Val { fn truthy(&self) -> bool { match self { Val::Str(s) => !s.is_empty(), Val::Num(n) => *n != 0, Val::Bool(b) => *b, Val::List(l) => !l.is_empty(), Val::Map(m) => !m.is_empty() } } }

#[derive(Debug)]
enum Node { Text(String), Var(String, Vec<String>), If(String, bool, Vec<Node>, Vec<Node>), For(String, String, Vec<Node>) }

#[derive(Debug)]
struct TplError { pos: usize, msg: String }
impl fmt::Display for TplError { fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { write!(f, "error at {}: {}", self.pos, self.msg) } }

enum Tok<'a> { Text(&'a str), Expr(&'a str, usize), Tag(&'a str, usize) }

fn lex(src: &str) -> Result<Vec<Tok<'_>>, TplError> {
    let mut out = vec![];
    let mut i = 0;
    while i < src.len() {
        let rest = &src[i..];
        let next = [rest.find("{{"), rest.find("{%")].into_iter().flatten().min();
        match next {
            None => { out.push(Tok::Text(rest)); break; }
            Some(0) => {
                let close = if rest.starts_with("{{") { "}}" } else { "%}" };
                let end = rest.find(close).ok_or(TplError { pos: i, msg: format!("unclosed {}", &rest[..2]) })?;
                let inner = rest[2..end].trim();
                out.push(if close == "}}" { Tok::Expr(inner, i) } else { Tok::Tag(inner, i) });
                i += end + 2;
            }
            Some(k) => { out.push(Tok::Text(&rest[..k])); i += k; }
        }
    }
    Ok(out)
}

fn parse<'a>(toks: &[Tok<'a>], pos: &mut usize, stop: &[&str]) -> Result<(Vec<Node>, Option<String>), TplError> {
    let mut nodes = vec![];
    while *pos < toks.len() {
        let t = &toks[*pos];
        *pos += 1;
        match t {
            Tok::Text(s) => nodes.push(Node::Text(s.to_string())),
            Tok::Expr(e, _) => {
                let mut parts = e.split('|').map(str::trim);
                let var = parts.next().unwrap().to_string();
                nodes.push(Node::Var(var, parts.map(String::from).collect()));
            }
            Tok::Tag(tag, at) => {
                let words: Vec<&str> = tag.split_whitespace().collect();
                match words.as_slice() {
                    [w] if stop.contains(w) => return Ok((nodes, Some(w.to_string()))),
                    ["if", cond] | ["if", "not", cond] => {
                        let neg = words.len() == 3;
                        let (then, end) = parse(toks, pos, &["else", "endif"])?;
                        let els = if end.as_deref() == Some("else") { parse(toks, pos, &["endif"])?.0 } else { vec![] };
                        if end.is_none() { return Err(TplError { pos: *at, msg: "unterminated if".into() }); }
                        nodes.push(Node::If(cond.to_string(), neg, then, els));
                    }
                    ["for", var, "in", list] => {
                        let (body, end) = parse(toks, pos, &["endfor"])?;
                        if end.is_none() { return Err(TplError { pos: *at, msg: "unterminated for".into() }); }
                        nodes.push(Node::For(var.to_string(), list.to_string(), body));
                    }
                    _ => return Err(TplError { pos: *at, msg: format!("unknown tag {:?}", tag) }),
                }
            }
        }
    }
    Ok((nodes, None))
}

struct Ctx<'p> { vars: BTreeMap<String, Val>, parent: Option<&'p Ctx<'p>> }
impl<'p> Ctx<'p> {
    fn get(&self, path: &str) -> Option<Val> {
        let mut segs = path.split('.');
        let first = segs.next()?;
        let mut v = match self.vars.get(first) { Some(v) => v.clone(), None => return self.parent.and_then(|p| p.get(path)) };
        for s in segs {
            v = match v { Val::Map(m) => m.get(s)?.clone(), Val::List(l) => l.get(s.parse::<usize>().ok()?)?.clone(), _ => return None };
        }
        Some(v)
    }
}

fn render(nodes: &[Node], ctx: &Ctx, out: &mut String) -> Result<(), TplError> {
    for n in nodes {
        match n {
            Node::Text(t) => out.push_str(t),
            Node::Var(name, filters) => {
                let mut v = ctx.get(name);
                for f in filters {
                    v = match (f.as_str(), v) {
                        ("upper", Some(x)) => Some(Val::Str(x.to_string().to_uppercase())),
                        ("len", Some(Val::List(l))) => Some(Val::Num(l.len() as i64)),
                        ("len", Some(x)) => Some(Val::Num(x.to_string().chars().count() as i64)),
                        (f, None) if f.starts_with("default:") => Some(Val::Str(f["default:".len()..].to_string())),
                        (f, x) if f.starts_with("default:") => x,
                        (other, _) => return Err(TplError { pos: 0, msg: format!("unknown filter {}", other) }),
                    };
                }
                out.push_str(&v.map(|x| x.to_string()).unwrap_or_else(|| format!("<missing {}>", name)));
            }
            Node::If(c, neg, then, els) => {
                let t = ctx.get(c).map_or(false, |v| v.truthy()) != *neg;
                render(if t { then } else { els }, ctx, out)?;
            }
            Node::For(var, list, body) => {
                let items = match ctx.get(list) { Some(Val::List(l)) => l, _ => vec![] };
                let n = items.len();
                for (i, it) in items.into_iter().enumerate() {
                    let mut vars = BTreeMap::new();
                    vars.insert(var.clone(), it);
                    vars.insert("loop".into(), Val::Map(BTreeMap::from([("index".to_string(), Val::Num(i as i64 + 1)), ("last".to_string(), Val::Bool(i + 1 == n))])));
                    let child = Ctx { vars, parent: Some(ctx) };
                    render(body, &child, out)?;
                }
            }
        }
    }
    Ok(())
}

fn run(src: &str, ctx: &Ctx) -> Result<String, TplError> {
    let toks = lex(src)?;
    let mut pos = 0;
    let (nodes, stray) = parse(&toks, &mut pos, &[])?;
    if let Some(s) = stray { return Err(TplError { pos, msg: format!("stray {}", s) }); }
    let mut out = String::new();
    render(&nodes, ctx, &mut out)?;
    Ok(out)
}

fn main() {
    let user = |n: &str, admin: bool, tags: &[&str]| Val::Map(BTreeMap::from([
        ("name".to_string(), Val::Str(n.into())),
        ("admin".to_string(), Val::Bool(admin)),
        ("tags".to_string(), Val::List(tags.iter().map(|t| Val::Str(t.to_string())).collect())),
    ]));
    let mut vars = BTreeMap::new();
    vars.insert("title".to_string(), Val::Str("Team Roster".into()));
    vars.insert("users".to_string(), Val::List(vec![user("ada", true, &["math", "eng"]), user("bob", false, &[]), user("cy", false, &["ops"])]));
    vars.insert("count".to_string(), Val::Num(3));
    let ctx = Ctx { vars, parent: None };
    let tpl = "# {{ title | upper }} ({{ users | len }} users, title len {{title|len}})\n{% for u in users %}{{ loop.index }}. {{ u.name }}{% if u.admin %} [admin]{% endif %}: {% if u.tags %}{% for t in u.tags %}{{ t }}{% if not loop.last %}/{% endif %}{% endfor %}{% else %}no tags{% endif %}\n{% endfor %}first tag: {{ users.0.tags.1 }} | missing: {{ nope }} | dflt: {{ nope | default:n/a }} | {{ count | default:x }}\n";
    match run(tpl, &ctx) { Ok(s) => print!("{}", s), Err(e) => println!("{}", e) }
    for bad in ["{{ open", "{% if x %}never closed", "{% endfor %}", "{% bogus tag %}", "{{ title | reverse }}", "{% for a in b %}x"] {
        println!("{:<28} => {}", format!("{:?}", bad), run(bad, &ctx).map_or_else(|e| e.to_string(), |s| format!("ok {:?}", s)));
    }
}
