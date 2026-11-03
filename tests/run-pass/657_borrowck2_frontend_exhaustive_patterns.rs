// Valid programs that a naive borrow checker / front end might reject.
// Each module is an independent program; main runs them in order.

mod p21_exhaustive_int_ranges {
    fn class_u8(n: u8) -> &'static str {
        match n { 0 => "zero", 1..=9 => "digit", 10..=99 => "two", 100..=u8::MAX => "three" }
    }
    fn sign(n: i32) -> i8 {
        match n { i32::MIN..=-1 => -1, 0 => 0, 1..=i32::MAX => 1 }
    }
    fn sign64(n: i64) -> i8 { match n { i64::MIN..0 => -1, 0 => 0, 1.. => 1 } }
    fn bucket(n: u16) -> u8 { match n { ..=255 => 0, 256..=1023 => 1, 1024.. => 2 } }
    fn small(n: i8) -> u8 { match n { -128..=-1 => 0, 0..=127 => 1 } }
    fn chars(c: char) -> u8 { match c { '\0'..='\u{D7FF}' => 0, '\u{E000}'..='\u{10FFFF}' => 1 } }
    fn tup(a: bool, b: u8) -> u8 {
        match (a, b) { (true, 0) => 0, (true, 1..) => 1, (false, x) => x }
    }
    pub fn main() {
        println!("{} {} {} {}", class_u8(0), class_u8(7), class_u8(42), class_u8(255));
        println!("{} {} {} {} {}", sign(i32::MIN), sign(0), sign(5), sign64(-9), sign64(i64::MAX));
        println!("{} {} {} {} {} {}", bucket(3), bucket(300), bucket(5000), small(-5), chars('z'), tup(false, 9));
    }
}

mod p22_exhaustive_slices_nested {
    #[derive(Debug, Clone, Copy)]
    enum Color { Red, Green, Blue }
    #[derive(Debug)]
    enum Shape { Circle(Color), Square { c: Color, filled: bool }, None }
    fn describe(s: &[i32]) -> String {
        match s {
            [] => "empty".into(),
            [x] => format!("one {}", x),
            [first, .., last] if first == last => format!("same ends {}", first),
            [first, middle @ .., last] => format!("{} {:?} {}", first, middle, last),
        }
    }
    fn bools(v: &[bool]) -> u8 { match v { [] => 0, [true, ..] => 1, [false] => 2, [false, _, ..] => 3 } }
    fn shape(s: &Shape) -> u8 {
        match s {
            Shape::Circle(Color::Red) => 1,
            Shape::Circle(Color::Green | Color::Blue) => 2,
            Shape::Square { c: Color::Red, filled: true } => 3,
            Shape::Square { filled: false, .. } => 4,
            Shape::Square { c: Color::Green, filled: true } | Shape::Square { c: Color::Blue, filled: true } => 5,
            Shape::None => 6,
        }
    }
    fn opt(o: Option<Option<bool>>) -> u8 { match o { None => 0, Some(None) => 1, Some(Some(true)) => 2, Some(Some(false)) => 3 } }
    fn refs(r: &Result<&str, &(i32, bool)>) -> i32 {
        match r { Ok(s) => s.len() as i32, Err(&(n, true)) => n, Err((n, false)) => -n }
    }
    fn arr(a: [u8; 3]) -> u8 { match a { [0, ..] => 0, [_, 0, _] => 1, [_, _, x] => x } }
    pub fn main() {
        println!("{} | {} | {} | {}", describe(&[]), describe(&[4]), describe(&[1, 2, 1]), describe(&[1, 2, 3, 4]));
        println!("{} {} {} {}", bools(&[]), bools(&[true]), bools(&[false]), bools(&[false, true]));
        let ss = [Shape::Circle(Color::Blue), Shape::Square { c: Color::Red, filled: true }, Shape::Square { c: Color::Blue, filled: false }, Shape::Square { c: Color::Green, filled: true }, Shape::None];
        let v: Vec<u8> = ss.iter().map(shape).collect();
        println!("{:?} {} {} {} {}", v, opt(Some(Some(false))), refs(&Ok("abc")), refs(&Err(&(4, false))), arr([1, 2, 7]));
    }
}

mod p48_const_generics_and_arrays {
    #[derive(Debug)]
    struct Matrix<const R: usize, const C: usize> { d: [[i32; C]; R] }
    impl<const R: usize, const C: usize> Matrix<R, C> {
        fn transpose(&self) -> Matrix<C, R> {
            let mut d = [[0; R]; C];
            for i in 0..R { for j in 0..C { d[j][i] = self.d[i][j]; } }
            Matrix { d }
        }
        fn row_mut(&mut self, i: usize) -> &mut [i32; C] { &mut self.d[i] }
    }
    fn sum<const N: usize>(a: [i32; N]) -> i32 { a.iter().sum() }
    const fn fib(n: u32) -> u64 { let mut a = 0; let mut b = 1; let mut i = 0; while i < n { let t = a + b; a = b; b = t; i += 1; } a }
    const F10: u64 = fib(10);
    static TABLE: [u64; 4] = [fib(1), fib(2), fib(3), F10];
    fn pair(a: [u8; 2]) -> u8 { match a { [0, x] | [x, 0] => x, [a, b] => a.max(b) } }
    pub fn main() {
        let mut m = Matrix { d: [[1, 2, 3], [4, 5, 6]] };
        m.row_mut(0)[2] = 30;
        let t = m.transpose();
        println!("{:?} {} {:?} {} {}", t, sum([1, 2, 3, 4]), TABLE, pair([0, 7]), pair([3, 5]));
    }
}

fn main() {
    p21_exhaustive_int_ranges::main();
    p22_exhaustive_slices_nested::main();
    p48_const_generics_and_arrays::main();
}
