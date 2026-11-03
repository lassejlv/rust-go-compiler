// split_inclusive, rsplitn, splitn, split_terminator, matches, rmatch_indices, split_once, trim_matches etc.
fn main() {
    let text = "line one\nline two\n\nline four\n";
    println!("{:?}", text.split_inclusive('\n').collect::<Vec<_>>());
    println!("{:?}", text.split('\n').collect::<Vec<_>>());
    println!("{:?}", text.split_terminator('\n').collect::<Vec<_>>());
    println!("{:?}", text.lines().collect::<Vec<_>>());
    println!("{:?}", "a\r\nb\nc\r\n".lines().collect::<Vec<_>>());
    let path = "/usr/local/lib/rustlib/x86_64/lib";
    println!("{:?}", path.rsplitn(2, '/').collect::<Vec<_>>());
    println!("{:?}", path.splitn(3, '/').collect::<Vec<_>>());
    println!("{:?}", path.rsplit('/').take(3).collect::<Vec<_>>());
    println!("{:?}", path.split_once("lib"));
    println!("{:?}", path.rsplit_once("lib"));
    println!("{:?}", "noseparator".split_once(':'));
    let csv = "a,b,,c,";
    println!("{:?}", csv.split(',').collect::<Vec<_>>());
    println!("{:?}", csv.rsplit(',').collect::<Vec<_>>());
    println!("{:?}", csv.split_terminator(',').collect::<Vec<_>>());
    println!("{:?}", csv.rsplit_terminator(',').collect::<Vec<_>>());
    println!("{:?}", "  a  b   c ".split(' ').collect::<Vec<_>>());
    println!("{:?}", "  a  b   c ".split_whitespace().rev().collect::<Vec<_>>());
    println!("{:?}", "  a\u{3000}b\tc ".split_ascii_whitespace().collect::<Vec<_>>());
    println!("{:?}", "abcXYZdefXYZ".split("XYZ").collect::<Vec<_>>());
    println!("{:?}", "a1b22c333d".split(char::is_numeric).collect::<Vec<_>>());
    println!("{:?}", "a1b22c333d".split(|c: char| c.is_ascii_digit()).filter(|s| !s.is_empty()).collect::<Vec<_>>());
    println!("{:?}", "one-two_three four".split(&['-', '_', ' '][..]).collect::<Vec<_>>());
    let hay = "abcabcababc";
    println!("{:?}", hay.matches("abc").count());
    println!("{:?}", hay.match_indices("ab").collect::<Vec<_>>());
    println!("{:?}", hay.rmatch_indices("abc").map(|(i, _)| i).collect::<Vec<_>>());
    println!("{:?}", "a1b2c3".matches(char::is_alphabetic).collect::<String>());
    println!("{:?}", "aaaa".matches("aa").count());
    println!("{:?} {:?}", hay.find("ca"), hay.rfind("ca"));
    println!("{:?} {:?}", hay.find(|c| c == 'z'), hay.find(char::is_lowercase));
    println!("{:?}", "xxhixx".trim_matches('x'));
    println!("{:?}", "xxhixx".trim_start_matches("xx"));
    println!("{:?}", "123abc456".trim_end_matches(char::is_numeric));
    println!("{:?}", "123abc456".trim_start_matches(|c: char| c.is_ascii_digit()));
    println!("{:?} {:?}", "prefix-body".strip_prefix("prefix-"), "body".strip_suffix("x"));
    println!("{:?}", "\t trim \n".trim_start());
    println!("{:?}", "\t trim \n".trim_end());
    println!("{:?}", "ab".repeat(3).replacen("ab", "X", 2));
    println!("{:?}", "Hello World".char_indices().filter(|(_, c)| c.is_uppercase()).collect::<Vec<_>>());
    println!("{:?}", "Mississippi".replace("ss", "SS").to_lowercase().split('s').collect::<Vec<_>>());
    let kv = "name=alice;age=30;city=paris;bad;x=";
    let parsed: Vec<(&str, &str)> = kv.split(';').filter_map(|p| p.split_once('=')).collect();
    println!("{:?}", parsed);
    let sentence = "The quick brown fox";
    let acronym: String = sentence.split(' ').filter_map(|w| w.chars().next()).collect();
    println!("{}", acronym);
    let camel = "parseHttpRequestHeader";
    let mut snake = String::new();
    for (i, c) in camel.char_indices() {
        if c.is_uppercase() {
            if i > 0 {
                snake.push('_');
            }
            snake.extend(c.to_lowercase());
        } else {
            snake.push(c);
        }
    }
    println!("{}", snake);
    let title: Vec<String> = "hello wide world".split(' ').map(|w| {
        let mut cs = w.chars();
        match cs.next() {
            Some(f) => f.to_uppercase().collect::<String>() + cs.as_str(),
            None => String::new(),
        }
    }).collect();
    println!("{}", title.join(" "));
    println!("{:?}", "abc".split("").collect::<Vec<_>>());
    println!("{:?}", "".split(',').collect::<Vec<_>>());
    println!("{:?}", "a,b,c".splitn(1, ',').collect::<Vec<_>>());
    println!("{:?}", "a,b,c".splitn(0, ',').collect::<Vec<_>>());
    println!("{} {}", "Hello".starts_with(char::is_uppercase), "hello".ends_with(&['o', 'x'][..]));
    println!("{}", "αβγ".contains('β'));
    println!("{:?}", "a-b-c".rsplitn(5, '-').collect::<Vec<_>>());
    println!("{:?}", "lo".cmp("hi"));
    println!("{:?}", "x=1, y=2".split(", ").map(|p| p.split('=').nth(1).unwrap()).collect::<Vec<_>>());
}
