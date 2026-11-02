fn main() {
    let sentence = "the quick  brown fox jumps over   the lazy dog";
    let words: Vec<&str> = sentence.split_whitespace().collect();
    println!("words = {:?}", words);
    println!("word count = {}", words.len());
    let parts: Vec<&str> = sentence.split(' ').collect();
    println!("split(' ') count = {} -> {:?}", parts.len(), parts);
    let csv = "name,age,city";
    for (i, field) in csv.split(',').enumerate() {
        println!("field {} = {}", i, field);
    }
    let kv = "key=value=more";
    let mut it = kv.splitn(2, '=');
    println!("splitn: {:?} {:?}", it.next(), it.next());
    println!("split_once: {:?}", kv.split_once('='));
    println!("rsplit_once: {:?}", kv.rsplit_once('='));
    println!("split_once none: {:?}", "abc".split_once(':'));
    let text = "line1\nline2\r\nline3\n";
    for (n, line) in text.lines().enumerate() {
        println!("{}: {}", n + 1, line);
    }
    let nums = "10 20 x 30 -5";
    let mut total = 0;
    for tok in nums.split(' ') {
        match tok.parse::<i32>() {
            Ok(v) => total += v,
            Err(_) => println!("skipping {:?}", tok),
        }
    }
    println!("total = {}", total);
    let floats: Vec<f64> = "1.5;2.25;3".split(';').map(|s| s.parse().unwrap()).collect();
    println!("floats = {:?}", floats);
    let multi: Vec<&str> = "a-b_c-d".split(|c| c == '-' || c == '_').collect();
    println!("multi = {:?}", multi);
    let terminated: Vec<&str> = "a;b;c;".split_terminator(';').collect();
    println!("split_terminator = {:?}", terminated);
    let rs: Vec<&str> = "1.2.3".rsplit('.').collect();
    println!("rsplit = {:?}", rs);
    let by_str: Vec<&str> = "one::two::three".split("::").collect();
    println!("by_str = {:?}", by_str);
    let empty_parts: Vec<&str> = ",a,,b,".split(',').collect();
    println!("empty_parts = {:?}", empty_parts);
    let b: bool = "true".parse().unwrap();
    let c: char = "x".parse().unwrap();
    let u: u64 = "18446744073709551615".parse().unwrap();
    println!("parsed: {} {} {}", b, c, u);
    println!("parse err: {:?}", "256".parse::<u8>().is_err());
    println!("parse neg: {:?}", "-12".parse::<i64>());
    println!("parse plus: {:?}", "+12".parse::<i32>());
}
