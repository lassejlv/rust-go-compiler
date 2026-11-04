// implicit format captures follow the hygiene of the format string literal
macro_rules! fmt_call_site {
    ($fmt:literal) => {{ let local = 5; let _ = local; format!($fmt) }};
}
macro_rules! fmt_def_site {
    () => {{ let local = 7; format!("def {local}") }};
}
macro_rules! fmt_both {
    ($v:ident) => {{ let local = 9; format!("{} {local} {}", $v, local + $v) }};
}
macro_rules! log {
    ($lvl:ident: $($arg:tt)+) => { println!("[{}] {}", stringify!($lvl), format!($($arg)+)) };
}
macro_rules! wrap_concat {
    ($name:ident) => { format!(concat!("<", "{", stringify!($name), "}", ">"), $name = 1) };
}
macro_rules! pad {
    ($s:expr, $w:literal) => { format!("{:>1$}|{2:<w2$}|", $s, $w, $s, w2 = $w) };
}

fn main() {
    let local = 1;
    println!("{}", fmt_call_site!("call {local}"));
    println!("{}", fmt_def_site!());
    let k = 2;
    println!("{}", fmt_both!(k));
    let user = "ann";
    let n = 3;
    log!(info: "user {user} logged in {n} times");
    log!(warn: "{} {:?}", user, (n, n * 2));
    log!(debug: "{user:>6}|{n:03}|{0}", 42);
    println!("{}", wrap_concat!(abc));
    println!("{}", pad!("ab", 5));
    let width = 6; let prec = 2;
    println!("{:width$.prec$}|{:>width$}|{:^+width$.prec$e}", 3.14159, "r", 1234.5);
    println!("{0:?} {0} {1:#?}", "q", (1,));
    println!("{:?}", format_args!("{}", 1).as_str());
    println!("{:?}", format_args!("static").as_str());
    let s = String::from("x");
    let r = &s;
    println!("{r} {r:?} {s:>3} {:p}", std::ptr::null::<u8>());
}
