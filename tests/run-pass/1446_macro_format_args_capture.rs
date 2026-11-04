// format_args! with inline captured identifiers produced by macros
macro_rules! show_var {
    ($v:ident) => { println!("{} = {}", stringify!($v), $v) };
}


macro_rules! fmt_lit {
    ($fmt:literal, $v:expr) => { format!($fmt, $v) };
}

macro_rules! cap_internal {
    () => {{ let inner = 5; format!("inner={inner}") }};
}

macro_rules! width {
    ($v:expr, $w:expr) => { format!("[{:>w$}]", $v, w = $w) };
}

macro_rules! my_println {
    ($($arg:tt)*) => { println!("> {}", format_args!($($arg)*)) };
}

fn main() {
    let alpha = 3;
    let beta = "b";
    show_var!(alpha);
    show_var!(beta);
    println!("{}", fmt_lit!("<{}>", 9));
    println!("{}", fmt_lit!("{:?}", "dbg"));
    println!("{}", cap_internal!());
    println!("{}", width!(42, 6));
    my_println!("x={alpha} y={beta:?} {}", 1);
    let w = 8; let p = 3; let val = 3.14159;
    my_println!("{val:>w$.p$}|{val:<w$}|{:^w$}|", "mid");
    my_println!("{0} {0:?} {name}", "s", name = alpha + 1);
    let v = vec![1, 2];
    my_println!("{v:?} {:#x} {:#b} {:08.3} {:+}", 255, 5, -1.5, 4);
    let s = format_args!("{} {}", 1, 2).to_string();
    println!("{s}");
    println!("{{literal}} {alpha:03}");
    println!("{:e} {:E}", 1234.5, 0.00012);
    eprintln!("to stderr {alpha}");
    print!("{}", format_args!("no newline {}\n", alpha));
}
