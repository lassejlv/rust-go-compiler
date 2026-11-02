use std::fmt::Write as FmtWrite;

fn main() {
    let name = "Ferris";
    let age = 7;
    println!("{name} is {age} years old");
    println!("{name:>10}|{age:<4}|");
    println!("{0} {1} {0} {1}", "a", "b");
    println!("{1} {0}", "first", "second");
    println!("{a} {b} {a}", a = 1, b = 2);
    println!("{} {name} {}", 1, 2, name = "mixed");
    println!("{{literal braces}} and {{{}}}", 5);
    println!("{{}}");
    let pi = 3.14159265;
    let w = 12;
    let p = 3;
    println!("[{pi:w$.p$}]");
    println!("[{pi:>w$}]");
    println!("[{:>1$.2$}]", pi, 10, 2);
    let s = format!("{}-{}", "x", 9);
    println!("format! -> {} (len {})", s, s.len());
    let mut buf = String::new();
    write!(buf, "{}+{}", 1, 2).unwrap();
    write!(&mut buf, "={}", 3).unwrap();
    println!("write! -> {}", buf);
    let v = vec![1, 2, 3];
    println!("{v:?}");
    println!("{:?} {v:?} {:?}", 0, 4);
    let debug_str = format!("{:?}", "hi");
    println!("debug_str = {} (len {})", debug_str, debug_str.len());
    let items = ["x", "y"];
    println!("{0:?} has {1} items; first={first}", items, items.len(), first = items[0]);
    let padded = format!("{:>5}", format!("{}", 42));
    println!("[{}]", padded);
    println!("{}", format_args!("{} {}", "via", "format_args"));
    eprintln!("this goes to stderr and is not compared");
    let width = 6;
    for (i, word) in ["a", "bb", "ccc"].iter().enumerate() {
        println!("{i}: [{word:^width$}]");
    }
    let x = 5;
    println!("{x} squared is {}", x * x);
    println!("{:5}|{:<5}|{:^5}|{:>5}|", 1, 2, 3, 4);
    print!("{}", format!("{}\n", "manual newline"));
    let multi = format!(
        "{}, {}, {}",
        "one",
        "two",
        "three",
    );
    println!("{multi}");
}
