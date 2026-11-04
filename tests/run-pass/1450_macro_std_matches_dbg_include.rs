// std macros: matches!, assert!, dbg!, write!, vec!, path-qualified macros, include_str!
use std::fmt::Write as _;

#[derive(Debug, Clone, PartialEq)]
enum Tok { Num(i64), Op(char), Ident(String) }

fn main() {
    let toks = vec![Tok::Num(3), Tok::Op('+'), Tok::Ident("x".into()), Tok::Num(-4)];
    let nums = toks.iter().filter(|t| matches!(t, Tok::Num(n) if *n > 0)).count();
    println!("{}", nums);
    println!("{}", matches!(toks[1], Tok::Op('+' | '-')));
    println!("{}", core::matches!(Some(3), Some(1..=2)));
    assert!(nums == 1, "expected one positive, got {}", nums);
    assert_eq!(toks.len(), 4, "len {len}", len = toks.len());
    assert_ne!(toks[0], toks[3]);
    debug_assert!(true);
    let d = dbg!(2 * 21);
    println!("{}", d);
    let (a, b) = dbg!(1, "two");
    println!("{} {}", a, b);
    let mut s = String::new();
    write!(s, "{}-{}", 1, 2).unwrap();
    writeln!(s, "!").unwrap();
    write!(&mut s, "{:?}", toks[2]).unwrap();
    println!("{}", s);
    let v = std::vec![0u8; 3];
    let w = ::std::vec![1, 2, 3];
    std::println!("{:?} {:?}", v, w);
    let src = include_str!("1450_macro_std_matches_dbg_include.rs");
    println!("{}", src.lines().next().unwrap());
    let bytes = include_bytes!("1450_macro_std_matches_dbg_include.rs");
    println!("{}", bytes[0] as char);
    println!("{:?}", option_env!("SURELY_NOT_SET_ENV_VAR_12345"));
    println!("{}", cfg!(debug_assertions) || true);
    println!("{} {}", cfg!(target_os = "linux"), cfg!(any(unix, windows)));
    println!("{}", cfg!(not(test)));
    let r: Result<i32, String> = (|| { let x: i32 = "5".parse().map_err(|_| "bad".to_string())?; Ok(x * 2) })();
    println!("{:?}", r);
    if false { unreachable!("never {}", 1); }
    if false { todo!(); }
    if false { unimplemented!("nope"); }
    let s2 = format!("{:?}", format_args!("{}", 5));
    println!("{}", s2);
    std::eprint!("err");
    let res = std::panic::catch_unwind(|| { assert!(1 + 1 == 3, "math is {}", "broken"); });
    println!("{}", res.is_err());
}
