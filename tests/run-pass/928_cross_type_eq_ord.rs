use std::cmp::Ordering;

#[derive(Debug, Clone, Copy)]
struct Inches(f64);
#[derive(Debug, Clone, Copy)]
struct Cm(f64);

impl PartialEq<Cm> for Inches { fn eq(&self, o: &Cm) -> bool { (self.0 * 2.54 - o.0).abs() < 1e-9 } }
impl PartialEq<Inches> for Cm { fn eq(&self, o: &Inches) -> bool { o == self } }
impl PartialOrd<Cm> for Inches { fn partial_cmp(&self, o: &Cm) -> Option<Ordering> { (self.0 * 2.54).partial_cmp(&o.0) } }
impl PartialEq for Inches { fn eq(&self, o: &Inches) -> bool { self.0 == o.0 } }

#[derive(Debug)]
struct Name(String);
impl PartialEq<str> for Name { fn eq(&self, o: &str) -> bool { self.0.eq_ignore_ascii_case(o) } }
impl PartialEq<&str> for Name { fn eq(&self, o: &&str) -> bool { self == *o } }
impl PartialEq<Name> for &str { fn eq(&self, o: &Name) -> bool { o == *self } }

#[derive(Debug, Clone, PartialEq, Eq)]
struct Version { major: u32, minor: u32, patch: u32, pre: Option<String> }
impl Ord for Version {
    fn cmp(&self, o: &Self) -> Ordering {
        (self.major, self.minor, self.patch).cmp(&(o.major, o.minor, o.patch)).then_with(|| match (&self.pre, &o.pre) {
            (None, None) => Ordering::Equal,
            (None, Some(_)) => Ordering::Greater,
            (Some(_), None) => Ordering::Less,
            (Some(a), Some(b)) => a.cmp(b),
        })
    }
}
impl PartialOrd for Version { fn partial_cmp(&self, o: &Self) -> Option<Ordering> { Some(self.cmp(o)) } }
fn v(s: &str) -> Version {
    let (core, pre) = match s.split_once('-') { Some((c, p)) => (c, Some(p.to_string())), None => (s, None) };
    let n: Vec<u32> = core.split('.').map(|x| x.parse().unwrap()).collect();
    Version { major: n[0], minor: n[1], patch: n[2], pre }
}
impl std::fmt::Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)?;
        if let Some(p) = &self.pre { write!(f, "-{}", p)?; }
        Ok(())
    }
}

fn count_eq<T: PartialEq<U>, U>(xs: &[T], target: &U) -> usize { xs.iter().filter(|x| *x == target).count() }
fn max_by_partial<T: PartialOrd + Copy>(xs: &[T]) -> Option<T> { xs.iter().copied().fold(None, |m, x| match m { None => Some(x), Some(y) => if x > y { Some(x) } else { Some(y) } }) }

fn main() {
    println!("{} {} {}", Inches(1.0) == Cm(2.54), Cm(5.08) == Inches(2.0), Inches(1.0) == Cm(3.0));
    println!("{} {} {:?}", Inches(1.0) < Cm(3.0), Inches(2.0) > Cm(3.0), Inches(1.0).partial_cmp(&Cm(f64::NAN)));
    println!("{}", count_eq(&[Inches(1.0), Inches(2.0), Inches(1.0)], &Cm(2.54)));
    let n = Name("Alice".into());
    println!("{} {} {}", n == *"alice", n == "ALICE", "bob" == n);
    let mut vs: Vec<Version> = ["1.2.3", "1.10.0", "1.2.3-beta", "0.9.9", "1.2.3-alpha", "1.2.10"].iter().map(|s| v(s)).collect();
    vs.sort();
    println!("{}", vs.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(" < "));
    println!("{:?}", vs.iter().max().map(|x| x.to_string()));
    println!("{:?}", v("2.0.0").cmp(&v("2.0.0-rc1")));
    println!("{}", v("1.0.0") == v("1.0.0"));
    println!("{:?} {:?}", max_by_partial(&[1.5, -2.0, 9.25]), max_by_partial::<i32>(&[]));
    println!("{:?}", [3.0, 1.0, 2.0f64].iter().copied().max_by(|a, b| a.partial_cmp(b).unwrap()));
    println!("{:?}", vs.iter().min_by_key(|x| (x.minor, x.patch)).map(|x| x.to_string()));
    println!("{:?}", 1.0f64.total_cmp(&-0.0));
    println!("{:?} {:?}", "abc".cmp("abd"), [1, 2, 3][..].cmp(&[1, 2][..]));
    println!("{:?}", (1, "z").partial_cmp(&(1, "a")));
    println!("{:?}", Some(3).cmp(&None));
    println!("{:?}", Ordering::Less.reverse().then(Ordering::Less));
}
