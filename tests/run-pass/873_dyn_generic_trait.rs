trait Transform<T> {
    fn apply(&self, x: T) -> T;
    fn name(&self) -> String;
}

struct AddN(i32);
struct MulN(i32);
struct Upper;
struct Suffix(String);

impl Transform<i32> for AddN {
    fn apply(&self, x: i32) -> i32 { x + self.0 }
    fn name(&self) -> String { format!("add{}", self.0) }
}
impl Transform<i32> for MulN {
    fn apply(&self, x: i32) -> i32 { x * self.0 }
    fn name(&self) -> String { format!("mul{}", self.0) }
}
impl Transform<String> for Upper {
    fn apply(&self, x: String) -> String { x.to_uppercase() }
    fn name(&self) -> String { "upper".into() }
}
impl Transform<String> for Suffix {
    fn apply(&self, x: String) -> String { x + &self.0 }
    fn name(&self) -> String { format!("suffix({})", self.0) }
}
// Same type implements Transform for two Ts
impl Transform<String> for MulN {
    fn apply(&self, x: String) -> String { x.repeat(self.0 as usize) }
    fn name(&self) -> String { format!("rep{}", self.0) }
}

struct Pipeline<T> {
    stages: Vec<Box<dyn Transform<T>>>,
}

impl<T: Clone + std::fmt::Debug> Pipeline<T> {
    fn new() -> Self { Pipeline { stages: Vec::new() } }
    fn add(mut self, t: Box<dyn Transform<T>>) -> Self {
        self.stages.push(t);
        self
    }
    fn run(&self, x: T) -> T {
        let mut cur = x;
        for s in &self.stages {
            let next = s.apply(cur.clone());
            println!("  {} : {:?} -> {:?}", s.name(), cur, next);
            cur = next;
        }
        cur
    }
}

fn apply_all<T>(ts: &[&dyn Transform<T>], x: T) -> T {
    ts.iter().fold(x, |acc, t| t.apply(acc))
}

fn main() {
    let p = Pipeline::new().add(Box::new(AddN(3))).add(Box::new(MulN(4))).add(Box::new(AddN(-1)));
    println!("result {}", p.run(2));
    let sp = Pipeline::new()
        .add(Box::new(Suffix("ab".into())))
        .add(Box::new(MulN(2)))
        .add(Box::new(Upper));
    println!("result {}", sp.run("x".to_string()));
    let a = AddN(1);
    let m = MulN(3);
    let v: Vec<&dyn Transform<i32>> = vec![&a, &m, &a];
    println!("{}", apply_all(&v, 5));
    let sv: Vec<&dyn Transform<String>> = vec![&m, &Upper];
    println!("{}", apply_all(&sv, "q".to_string()));
    let names: Vec<String> = p.stages.iter().map(|s| s.name()).collect();
    println!("{:?}", names);
}
