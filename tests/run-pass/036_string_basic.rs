fn greet(name: &str) -> String {
    format!("Hello, {}!", name)
}

fn takes_ownership(s: String) -> usize {
    s.len()
}

fn main() {
    let mut s = String::new();
    println!("empty: '{}' len={} is_empty={}", s, s.len(), s.is_empty());
    s.push('a');
    s.push_str("bc");
    s.push_str("def");
    println!("s = {} len = {}", s, s.len());
    let lit: &str = "literal";
    let owned: String = lit.to_string();
    let owned2 = String::from("from");
    let owned3 = "into".to_owned();
    println!("{} {} {} {}", lit, owned, owned2, owned3);
    let joined = owned + " " + &owned2 + " " + lit;
    println!("joined = {}", joined);
    let mut acc = String::from("x");
    acc += "y";
    acc += &String::from("z");
    println!("acc = {}", acc);
    println!("{}", greet("Ferris"));
    let n = takes_ownership(greet("World"));
    println!("len of greeting = {}", n);
    let cloned = acc.clone();
    acc.push('!');
    println!("acc = {}, cloned = {}", acc, cloned);
    let num = 42.to_string();
    let f = 2.5.to_string();
    println!("num = {}, f = {}, sum len = {}", num, f, num.len() + f.len());
    let multi = "line one
line two";
    println!("{}", multi);
    let cont = "abc\
               def";
    println!("{}", cont);
    let raw = r"C:\path\no\escape";
    println!("{}", raw);
    let raw2 = r#"she said "hi""#;
    println!("{}", raw2);
    println!("eq: {} {}", acc == "xyz!", "abc" == String::from("abc"));
    let s3 = String::with_capacity(10);
    println!("with_capacity len = {}", s3.len());
    let utf = String::from("héllo");
    println!("utf len bytes = {}, chars = {}", utf.len(), utf.chars().count());
}
