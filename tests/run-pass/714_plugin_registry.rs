// Plugin system: Box<dyn Plugin> registries, factories stored as boxed
// closures and fn pointers, priority ordering, capabilities via default
// methods and downcasting through as_any, and a pipeline of transforms.
use std::any::Any;
use std::collections::BTreeMap;
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
enum Value { Int(i64), Text(String), List(Vec<Value>) }

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Value::Int(i) => write!(f, "{}", i),
            Value::Text(s) => write!(f, "{:?}", s),
            Value::List(v) => {
                write!(f, "[")?;
                for (i, x) in v.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{}", x)?;
                }
                write!(f, "]")
            }
        }
    }
}

#[derive(Debug)]
enum PluginError { Unsupported(&'static str, String), Failed(String) }

trait Plugin {
    fn name(&self) -> &str;
    fn priority(&self) -> i32 { 0 }
    fn apply(&mut self, v: Value) -> Result<Value, PluginError>;
    fn describe(&self) -> String { format!("{} (prio {})", self.name(), self.priority()) }
    fn as_any(&self) -> &dyn Any;
}

struct Doubler;
impl Plugin for Doubler {
    fn name(&self) -> &str { "doubler" }
    fn apply(&mut self, v: Value) -> Result<Value, PluginError> {
        match v {
            Value::Int(i) => i.checked_mul(2).map(Value::Int).ok_or(PluginError::Failed("overflow".into())),
            Value::Text(s) => Ok(Value::Text(s.repeat(2))),
            Value::List(items) => items.into_iter().map(|x| self.apply(x)).collect::<Result<Vec<_>, _>>().map(Value::List),
        }
    }
    fn as_any(&self) -> &dyn Any { self }
}

struct Counter { calls: u32, prio: i32 }
impl Plugin for Counter {
    fn name(&self) -> &str { "counter" }
    fn priority(&self) -> i32 { self.prio }
    fn apply(&mut self, v: Value) -> Result<Value, PluginError> {
        self.calls += 1;
        Ok(v)
    }
    fn as_any(&self) -> &dyn Any { self }
}

struct Upper { label: String }
impl Plugin for Upper {
    fn name(&self) -> &str { &self.label }
    fn priority(&self) -> i32 { 10 }
    fn apply(&mut self, v: Value) -> Result<Value, PluginError> {
        match v {
            Value::Text(s) => Ok(Value::Text(s.to_uppercase())),
            Value::List(items) => Ok(Value::List(items.into_iter().map(|x| match x {
                Value::Text(s) => Value::Text(s.to_uppercase()),
                o => o,
            }).collect())),
            other => Err(PluginError::Unsupported("upper", other.to_string())),
        }
    }
    fn as_any(&self) -> &dyn Any { self }
}

struct Closure<F: FnMut(Value) -> Value> { f: F, tag: &'static str }
impl<F: FnMut(Value) -> Value + 'static> Plugin for Closure<F> {
    fn name(&self) -> &str { self.tag }
    fn priority(&self) -> i32 { -5 }
    fn apply(&mut self, v: Value) -> Result<Value, PluginError> { Ok((self.f)(v)) }
    fn as_any(&self) -> &dyn Any { self }
}

type Factory = Box<dyn Fn(&str) -> Box<dyn Plugin>>;

#[derive(Default)]
struct Registry {
    factories: BTreeMap<String, Factory>,
    active: Vec<Box<dyn Plugin>>,
}

impl Registry {
    fn register(&mut self, key: &str, f: Factory) { self.factories.insert(key.to_string(), f); }
    fn register_fn(&mut self, key: &str, f: fn(&str) -> Box<dyn Plugin>) { self.register(key, Box::new(f)); }
    fn enable(&mut self, key: &str, arg: &str) -> Result<(), String> {
        let f = self.factories.get(key).ok_or_else(|| format!("no plugin '{}'", key))?;
        self.active.push(f(arg));
        self.active.sort_by_key(|p| std::cmp::Reverse(p.priority()));
        Ok(())
    }
    fn run(&mut self, v: Value) -> (Value, Vec<String>) {
        let mut cur = v;
        let mut errs = Vec::new();
        for p in self.active.iter_mut() {
            match p.apply(cur.clone()) {
                Ok(n) => cur = n,
                Err(e) => errs.push(format!("{}: {:?}", p.name(), e)),
            }
        }
        (cur, errs)
    }
    fn find<T: 'static>(&self) -> Vec<&T> {
        self.active.iter().filter_map(|p| p.as_any().downcast_ref::<T>()).collect()
    }
}

fn make_upper(arg: &str) -> Box<dyn Plugin> { Box::new(Upper { label: format!("upper[{}]", arg) }) }

fn main() {
    let mut reg = Registry::default();
    reg.register("double", Box::new(|_| Box::new(Doubler)));
    reg.register("count", Box::new(|a| Box::new(Counter { calls: 0, prio: a.parse().unwrap_or(0) })));
    reg.register_fn("upper", make_upper);
    let offset = 7;
    reg.register("offset", Box::new(move |_| Box::new(Closure {
        tag: "offset",
        f: move |v| match v { Value::Int(i) => Value::Int(i + offset), o => o },
    })));
    for (k, a) in [("double", ""), ("count", "3"), ("upper", "x"), ("offset", ""), ("missing", ""), ("count", "-9")] {
        if let Err(e) = reg.enable(k, a) {
            println!("enable error: {}", e);
        }
    }
    println!("factories: {:?}", reg.factories.keys().collect::<Vec<_>>());
    for p in &reg.active {
        println!("  {}", p.describe());
    }
    let inputs = vec![
        Value::Int(20),
        Value::Text("ab".into()),
        Value::List(vec![Value::Int(1), Value::Text("q".into()), Value::List(vec![Value::Int(-4)])]),
        Value::Int(i64::MAX / 2 + 1),
    ];
    for v in inputs {
        let shown = v.to_string();
        let (out, errs) = reg.run(v);
        println!("{} => {} errors={:?}", shown, out, errs);
    }
    let counters = reg.find::<Counter>();
    println!("counters: {:?}", counters.iter().map(|c| (c.prio, c.calls)).collect::<Vec<_>>());
    println!("uppers: {:?}", reg.find::<Upper>().iter().map(|u| u.label.as_str()).collect::<Vec<_>>());
    println!("doublers: {}", reg.find::<Doubler>().len());
}
