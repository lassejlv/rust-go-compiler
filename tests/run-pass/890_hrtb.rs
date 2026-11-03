fn apply_to_all<F>(items: &[String], f: F) -> Vec<&str>
where
    F: for<'a> Fn(&'a str) -> &'a str,
{
    items.iter().map(|s| f(s.as_str())).collect()
}

fn first_word(s: &str) -> &str { s.split(' ').next().unwrap_or("") }
fn trim_x(s: &str) -> &str { s.trim_matches('x') }

struct Selector<F> { f: F }
impl<F> Selector<F>
where
    F: for<'a> Fn(&'a [i32]) -> Option<&'a i32>,
{
    fn pick<'b>(&self, v: &'b [i32]) -> Option<&'b i32> { (self.f)(v) }
}

fn selector<F>(f: F) -> Selector<F> where F: for<'a> Fn(&'a [i32]) -> Option<&'a i32> { Selector { f } }

fn call_with_ref<F>(f: F) -> usize where F: for<'a> FnOnce(&'a Vec<u8>) -> usize {
    let v = vec![1, 2, 3, 4];
    f(&v)
}

trait Getter<T> {
    fn get<'a>(&self, t: &'a T) -> &'a str;
}
struct NameGetter;
struct Person { name: String, city: String }
impl Getter<Person> for NameGetter { fn get<'a>(&self, p: &'a Person) -> &'a str { &p.name } }
struct CityGetter;
impl Getter<Person> for CityGetter { fn get<'a>(&self, p: &'a Person) -> &'a str { &p.city } }

fn project<'p, T>(xs: &'p [T], g: &dyn Getter<T>) -> Vec<&'p str> { xs.iter().map(|x| g.get(x)).collect() }

fn compose_ref<F, G>(f: F, g: G) -> impl Fn(&String) -> &str
where
    F: for<'a> Fn(&'a String) -> &'a str,
    G: for<'a> Fn(&'a str) -> &'a str,
{
    move |a| g(f(a))
}

fn boxed_hrtb() -> Vec<Box<dyn for<'a> Fn(&'a str) -> &'a str>> {
    vec![Box::new(first_word), Box::new(trim_x), Box::new(|s: &str| &s[s.len() / 2..])]
}

fn main() {
    let items = vec!["hello world".to_string(), "xxabcxx".to_string(), "one".to_string()];
    println!("{:?}", apply_to_all(&items, first_word));
    println!("{:?}", apply_to_all(&items, trim_x));
    println!("{:?}", apply_to_all(&items, |s| if s.len() > 3 { &s[..3] } else { s }));
    let sel = selector(|v| v.iter().max());
    let sel2 = selector(|v| v.iter().find(|x| **x < 0));
    let data = [3, -1, 9, -7];
    println!("{:?} {:?} {:?}", sel.pick(&data), sel2.pick(&data), sel2.pick(&[1, 2]));
    println!("{}", call_with_ref(|v| v.len() * 10));
    println!("{}", call_with_ref(|v| v.iter().map(|&x| x as usize).sum()));
    let ps = vec![Person { name: "Ann".into(), city: "Oslo".into() }, Person { name: "Bob".into(), city: "Rome".into() }];
    println!("{:?} {:?}", project(&ps, &NameGetter), project(&ps, &CityGetter));
    let c = compose_ref(|s: &String| s.as_str(), |s: &str| first_word(s));
    let owned = String::from("alpha beta");
    println!("{}", c(&owned));
    for f in boxed_hrtb() { println!("[{}]", f("xxsplit hairsx")); }
}
