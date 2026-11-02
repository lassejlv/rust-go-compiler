// Built-in macros: stringify!, concat!, line!/column!-free metadata (we avoid file paths),
// include-less static tables, env!-free version consts, matches!, assert! variants, debug_assert!,
// format_args!, and const string concatenation via macros.

macro_rules! version {
    ($maj:literal . $min:literal . $patch:literal) => {
        concat!("v", stringify!($maj), ".", stringify!($min), ".", stringify!($patch))
    };
}

const VERSION: &str = version!(1 . 4 . 2);
const BANNER: &str = concat!("app ", "/", " build ", 42, " ", true, " ", 'x', " ", 2.5);

macro_rules! show_expr {
    ($e:expr) => {
        println!("{:<28} = {:?}", stringify!($e), $e)
    };
}

macro_rules! field_names {
    ($s:ident { $($f:ident),* }) => {
        [$(concat!(stringify!($s), ".", stringify!($f))),*]
    };
}

macro_rules! const_str_array {
    ($name:ident = [$($x:ident)*]) => {
        const $name: &[&str] = &[$(stringify!($x)),*];
    };
}
const_str_array!(KEYWORDS = [fn let mut impl trait where async await]);

macro_rules! assert_all {
    ($($cond:expr),+ $(,)?) => {
        $( assert!($cond, "assertion failed: {}", stringify!($cond)); )+
    };
}

macro_rules! tokens_of {
    ($($t:tt)*) => { stringify!($($t)*) };
}

#[derive(Debug, PartialEq)]
enum Tok {
    Num(i64),
    Ident(String),
    Op(char),
}

fn main() {
    println!("{}", VERSION);
    println!("{}", BANNER);
    println!("{}", concat!(1, 2, 3, "-", -4));
    show_expr!(1 + 2 * 3);
    show_expr!("abc".len() as i32 - 5);
    show_expr!(vec![1, 2, 3].iter().rev().collect::<Vec<_>>());
    show_expr!(matches!(Some(3), Some(x) if x > 2));
    show_expr!(matches!('q', 'a'..='m' | 'z'));
    show_expr!(Tok::Op('+') == Tok::Op('+'));
    println!("{:?}", field_names!(Point { x, y, z }));
    println!("{:?} ({} keywords)", KEYWORDS, KEYWORDS.len());
    println!("{}", tokens_of!(struct S < T : Clone > { a : Vec < T > }));
    println!("{}", tokens_of!(a + b * c));
    println!("{}", stringify!(if x { y } else { z }));
    println!("[{}]", stringify!());

    let toks = vec![Tok::Num(3), Tok::Ident("x".into()), Tok::Op('*'), Tok::Num(-1)];
    let nums = toks.iter().filter(|t| matches!(t, Tok::Num(n) if *n > 0)).count();
    let idents: Vec<&str> = toks.iter().filter_map(|t| match t {
        Tok::Ident(s) => Some(s.as_str()),
        _ => None,
    }).collect();
    println!("positive nums={} idents={:?}", nums, idents);

    assert_all!(1 < 2, "a" < "b", VERSION.starts_with('v'));
    assert_eq!(VERSION, "v1.4.2", "version mismatch");
    assert_ne!(BANNER.len(), 0);
    debug_assert!(KEYWORDS.contains(&"impl"));
    let r = std::panic::catch_unwind(|| {
        assert_all!(true, 2 + 2 == 5);
    });
    println!("caught assertion: {}", r.is_err());

    let s = format!("{}", format_args!("{}-{}", stringify!(alpha), concat!("be", "ta")));
    println!("{}", s);
    let raw = r#"raw "quoted" \n not escaped"#;
    println!("{} {}", raw, raw.len());
    let bytes = b"bytes\x41\n";
    println!("{:?} {}", bytes, bytes.len());
    let multi = "line one \
                 continues here";
    println!("{}", multi);
    println!("{}", concat!("tab:\t", "end"));
}
