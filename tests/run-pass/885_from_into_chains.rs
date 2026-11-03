use std::convert::TryFrom;
use std::fmt::Debug;

#[derive(Debug, Clone, PartialEq)]
struct Celsius(f64);
#[derive(Debug, Clone, PartialEq)]
struct Fahrenheit(f64);
impl From<Celsius> for Fahrenheit { fn from(c: Celsius) -> Self { Fahrenheit(c.0 * 9.0 / 5.0 + 32.0) } }
impl From<Fahrenheit> for Celsius { fn from(f: Fahrenheit) -> Self { Celsius((f.0 - 32.0) * 5.0 / 9.0) } }

#[derive(Debug)]
struct Wrapper<T>(T);
impl<T> From<T> for Wrapper<T> { fn from(t: T) -> Self { Wrapper(t) } }

#[derive(Debug, PartialEq)]
struct Even(u32);
#[derive(Debug, PartialEq)]
enum EvenErr { Odd(u32), TooBig }
impl TryFrom<u32> for Even {
    type Error = EvenErr;
    fn try_from(v: u32) -> Result<Self, EvenErr> {
        if v > 1000 { Err(EvenErr::TooBig) } else if v % 2 == 1 { Err(EvenErr::Odd(v)) } else { Ok(Even(v)) }
    }
}
impl TryFrom<i64> for Even {
    type Error = String;
    fn try_from(v: i64) -> Result<Self, String> {
        let u = u32::try_from(v).map_err(|e| format!("conv: {}", e))?;
        Even::try_from(u).map_err(|e| format!("{:?}", e))
    }
}

#[derive(Debug)]
struct Point<T> { x: T, y: T }
impl<T> From<(T, T)> for Point<T> { fn from((x, y): (T, T)) -> Self { Point { x, y } } }
impl<T> From<[T; 2]> for Point<T> {
    fn from(a: [T; 2]) -> Self { let [x, y] = a; Point { x, y } }
}
impl<T> From<Point<T>> for (T, T) { fn from(p: Point<T>) -> Self { (p.x, p.y) } }

fn to_points<T, U: Into<Point<T>>>(items: Vec<U>) -> Vec<Point<T>> { items.into_iter().map(Into::into).collect() }

fn convert_all<A, B>(v: Vec<A>) -> Vec<B> where A: Into<B> { v.into_iter().map(|a| a.into()).collect() }

fn try_all<T, U>(v: Vec<T>) -> Result<Vec<U>, U::Error> where U: TryFrom<T> {
    v.into_iter().map(U::try_from).collect()
}

fn main() {
    let f: Fahrenheit = Celsius(100.0).into();
    println!("{:?}", f);
    let c = Celsius::from(Fahrenheit(212.0));
    println!("{:?} {}", c, c == Celsius(100.0));
    let w: Wrapper<&str> = "hi".into();
    let w2 = Wrapper::from(vec![1, 2]);
    println!("{:?} {:?}", w, w2);
    println!("{:?}", Even::try_from(4u32));
    println!("{:?}", Even::try_from(5u32));
    println!("{:?}", Even::try_from(5000u32));
    println!("{:?}", Even::try_from(-3i64));
    println!("{:?}", Even::try_from(7i64));
    println!("{:?}", Even::try_from(8i64));
    let r: Result<Even, _> = 10u32.try_into();
    println!("{:?}", r);
    let pts = to_points(vec![(1, 2), (3, 4)]);
    println!("{:?}", pts);
    let pts2: Vec<Point<f32>> = to_points(vec![[0.5, 1.5]]);
    println!("{:?}", pts2);
    let t: (i32, i32) = Point { x: 9, y: 8 }.into();
    println!("{:?}", t);
    let wide: Vec<i64> = convert_all(vec![1i32, -2, 3]);
    println!("{:?}", wide);
    let strs: Vec<String> = convert_all(vec!["a", "b"]);
    println!("{:?}", strs);
    let fs: Vec<Fahrenheit> = convert_all(vec![Celsius(0.0), Celsius(-40.0)]);
    println!("{:?}", fs);
    println!("{:?}", try_all::<u32, Even>(vec![2, 4, 6]));
    println!("{:?}", try_all::<u32, Even>(vec![2, 3, 6]));
    println!("{:?}", try_all::<i32, u8>(vec![1, 255, 256]).is_err());
    println!("{:?}", try_all::<i32, u8>(vec![1, 255]));
}
