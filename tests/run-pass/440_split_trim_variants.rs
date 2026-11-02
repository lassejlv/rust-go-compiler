// str splitting and trimming: split with char/str/closure/char-slice patterns, splitn, rsplitn,
// split_terminator, rsplit_terminator, split_inclusive, split_once/rsplit_once, split_ascii_whitespace,
// lines with \r\n, matches/match_indices/rmatch_indices, trim_matches/trim_start_matches,
// strip_prefix/strip_suffix, and a CSV-ish parser with quoted fields.
fn parse_csv_line(line: &str) -> Vec<String> {
    let mut fields = Vec::new();
    let mut cur = String::new();
    let mut in_q = false;
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        match (c, in_q) {
            ('"', true) if chars.peek() == Some(&'"') => {
                cur.push('"');
                chars.next();
            }
            ('"', _) => in_q = !in_q,
            (',', false) => fields.push(std::mem::take(&mut cur)),
            (c, _) => cur.push(c),
        }
    }
    fields.push(cur);
    fields
}

fn main() {
    let s = "a,b,,c,";
    println!("{:?}", s.split(',').collect::<Vec<_>>());
    println!("{:?}", s.split_terminator(',').collect::<Vec<_>>());
    println!("{:?}", s.rsplit_terminator(',').collect::<Vec<_>>());
    println!("{:?}", s.splitn(3, ',').collect::<Vec<_>>());
    println!("{:?}", s.rsplitn(3, ',').collect::<Vec<_>>());
    println!("{:?}", s.split_inclusive(',').collect::<Vec<_>>());
    println!("{:?}", s.rsplit(',').collect::<Vec<_>>());
    println!("{:?}", "".split(',').collect::<Vec<_>>());
    println!("{:?}", "abc".split("").collect::<Vec<_>>());

    let t = "one--two---three--";
    println!("{:?}", t.split("--").collect::<Vec<_>>());
    println!("{:?}", t.split(|c: char| c == '-' || c == 'w').filter(|p| !p.is_empty()).collect::<Vec<_>>());
    println!("{:?}", "a1b22c333d".split(char::is_numeric).collect::<Vec<_>>());
    println!("{:?}", "x;y:z,w".split(&[';', ':', ','][..]).collect::<Vec<_>>());
    println!("{:?}", "x;y:z".split([';', ':']).collect::<Vec<_>>());

    println!("{:?} {:?}", "key=value=more".split_once('='), "key=value=more".rsplit_once('='));
    println!("{:?} {:?}", "novalue".split_once('='), "a::b::c".split_once("::"));

    let ws = "  hello \t world\n\u{a0}nbsp  \u{2003}em ";
    println!("{:?}", ws.split_whitespace().collect::<Vec<_>>());
    println!("{:?}", ws.split_ascii_whitespace().collect::<Vec<_>>());
    println!("{:?}", ws.split(' ').collect::<Vec<_>>());

    let text = "line1\nline2\r\nline3\n\nline5\r\n";
    println!("{:?}", text.lines().collect::<Vec<_>>());
    println!("{:?}", "no newline".lines().count());

    let hay = "abcabcab";
    println!("{:?}", hay.matches("ab").collect::<Vec<_>>());
    println!("{:?}", hay.match_indices("abc").collect::<Vec<_>>());
    println!("{:?}", hay.rmatch_indices('b').collect::<Vec<_>>());
    println!("{:?}", "aaaa".matches("aa").count());
    println!("{:?} {:?} {:?}", hay.find("ca"), hay.rfind("ab"), hay.find(|c: char| c > 'b'));

    let padded = "xx--hello--xx";
    println!("[{}] [{}] [{}]", padded.trim_matches('x'), padded.trim_start_matches("xx-"), padded.trim_end_matches(&['x', '-'][..]));
    println!("[{}] [{}] [{}]", "  hi  ".trim(), "  hi  ".trim_start(), "  hi  ".trim_end());
    println!("[{}]", "123abc456".trim_matches(char::is_numeric));
    println!("[{}]", "\u{3000}\t full width\u{3000}".trim());
    println!("{:?} {:?}", "prefix_rest".strip_prefix("prefix_"), "file.tar.gz".strip_suffix(".gz"));
    println!("{:?} {:?}", "abc".strip_prefix('b'), "abc".strip_prefix(|c: char| c.is_alphabetic()));

    let lines = [
        r#"name,age,city"#,
        r#""Smith, John",42,"New ""York""""#,
        r#"plain,,"#,
        r#""a""b",x,"multi, comma, field""#,
    ];
    for l in lines {
        println!("{:?}", parse_csv_line(l));
    }

    let path = "/usr/local/lib/rustlib/x86_64/lib.rs";
    let comps: Vec<&str> = path.split('/').filter(|c| !c.is_empty()).collect();
    let file = path.rsplit('/').next().unwrap();
    let (stem, ext) = file.rsplit_once('.').unwrap_or((file, ""));
    println!("{:?} file={} stem={} ext={}", comps, file, stem, ext);
    let dir = &path[..path.rfind('/').unwrap()];
    println!("dir={} depth={}", dir, dir.matches('/').count());

    let kv = "a=1; b = 2 ;c=  3;;d";
    let pairs: Vec<(String, Option<i32>)> = kv
        .split(';')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| match s.split_once('=') {
            Some((k, v)) => (k.trim().to_string(), v.trim().parse().ok()),
            None => (s.to_string(), None),
        })
        .collect();
    println!("{:?}", pairs);
    let chars_split: Vec<String> = "hello world".split('o').map(|p| p.chars().rev().collect()).collect();
    println!("{:?}", chars_split);
}
