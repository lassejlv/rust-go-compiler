// str APIs with every Pattern kind: char, &str, &String, char arrays/slices,
// closures and fn items (char::is_whitespace); split variants (terminator,
// inclusive, n, r*), match_indices/rmatch_indices, trim_*_matches,
// strip_prefix/suffix, replacen, and slice APIs on byte/char vectors.
fn main() {
    let s = "alpha, beta;gamma ,delta;; epsilon";
    println!("{:?}", s.split([',', ';']).map(str::trim).collect::<Vec<_>>());
    println!("{:?}", s.split(&[',', ';'][..]).filter(|p| !p.is_empty()).count());
    println!("{:?}", s.split(|c: char| c == ',' || c.is_whitespace()).filter(|x| !x.is_empty()).collect::<Vec<_>>());
    println!("{:?}", s.splitn(3, char::is_whitespace).collect::<Vec<_>>());
    println!("{:?}", s.rsplitn(2, ';').collect::<Vec<_>>());
    println!("{:?}", "a,b,c,".split_terminator(',').collect::<Vec<_>>());
    println!("{:?}", "a,b,c,".rsplit_terminator(',').collect::<Vec<_>>());
    println!("{:?}", "line1\nline2\n\nline4".split_inclusive('\n').collect::<Vec<_>>());
    println!("{:?}", "x--y--z".rsplit("--").collect::<Vec<_>>());
    let pat = String::from("an");
    println!("{:?} {:?}", "banana".match_indices(&pat).collect::<Vec<_>>(), "banana".rmatch_indices("ana").collect::<Vec<_>>());
    println!("{:?}", "a1b22c333".matches(char::is_numeric).collect::<String>());
    println!("{:?}", "a1b22c333".match_indices(|c: char| c.is_alphabetic()).map(|(i, _)| i).collect::<Vec<_>>());
    println!("{:?} {:?}", "xxabcxx".trim_matches('x'), "123abc456".trim_start_matches(char::is_numeric));
    println!("{:?} {:?}", "ababcab".trim_start_matches("ab"), "ababcab".trim_end_matches("ab"));
    println!("{:?} {:?}", "  \t padded \n".trim_start(), "  \t padded \n".trim_end());
    println!("{:?} {:?} {:?}", "prefix-body".strip_prefix("prefix-"), "body.rs".strip_suffix(".rs"), "nope".strip_prefix('x'));
    println!("{:?} {:?}", "#tag".strip_prefix(['#', '@']), "@user".strip_prefix(|c: char| !c.is_alphanumeric()));
    println!("{}", "a-b-c-d".replacen('-', "+", 2));
    println!("{}", "Hello World".replace(char::is_uppercase, "_"));
    println!("{:?} {:?} {:?}", "hello".find('l'), "hello".rfind('l'), "hello".find(|c: char| c > 'h'));
    println!("{:?} {:?}", "hello".find("ll"), "hello".find(&String::from("lo")));
    println!("{} {} {}", "Hello".contains(char::is_uppercase), "hello".starts_with(['h', 'j']), "hello".ends_with(|c: char| c == 'o'));
    println!("{:?}", "key=value=more".split_once('=').map(|(k, v)| (k.len(), v)));
    println!("{:?}", "key=value=more".rsplit_once("=").unwrap());
    println!("{:?}", "AbCdE".char_indices().rev().filter(|(_, c)| c.is_uppercase()).collect::<Vec<_>>());
    println!("{:?}", "a b".bytes().rposition(|b| b == b' '));
    let digits: String = "a1b2c3".chars().filter(char::is_ascii_digit).collect();
    println!("{} {}", digits, "ab".repeat(3).to_owned() + &"-".repeat(2));
    let mut parts: Vec<&str> = "pear fig banana kiwi apple".split(' ').collect();
    parts.sort_unstable_by_key(|p| (p.len(), *p));
    println!("{:?}", parts);
    println!("{:?}", "one two  three".split(' ').collect::<Vec<_>>());
    println!("{:?}", "one two  three".split_whitespace().rev().collect::<Vec<_>>());
    println!("{:?}", "Mary had a little lamb".split(char::is_whitespace).map(|w| w.chars().next().unwrap()).collect::<String>());
    println!("{:?}", "comma,separated".split(",").chain("more".split(",")).collect::<Vec<_>>());
    println!("{:?}", "lines\r\nwith\r\ncrlf\n".lines().collect::<Vec<_>>());
    println!("{:?}", "αβγ,δ".split(',').map(|p| p.chars().count()).collect::<Vec<_>>());
    println!("{:?}", "a=1;b=2;c".split(';').map(|kv| kv.split_once('=').unwrap_or((kv, "?"))).collect::<Vec<_>>());

    let mut bytes = *b"hello world";
    bytes[..5].make_ascii_uppercase();
    bytes.swap(0, 10);
    println!("{}", String::from_utf8_lossy(&bytes));
    let mut v: Vec<u8> = (1..=10).collect();
    v[2..6].fill(0);
    let (head, tail) = v.split_first().unwrap();
    println!("{} {:?} {:?}", head, tail, v.split_last().map(|(l, _)| l));
    v.chunks_mut(3).for_each(|c| c.reverse());
    println!("{:?}", v);
    let (a, b) = v.split_at_mut(5);
    a.swap_with_slice(b);
    println!("{:?}", v);
    let mut w = vec![0u8; 4];
    w.copy_from_slice(&v[3..7]);
    w.fill_with(Default::default);
    println!("{:?} {:?}", w, [[1, 2], [3, 4]].concat());
    println!("{:?} {:?}", [vec![1], vec![2, 3]].join(&0), ["a", "b", "c"].join("/"));
    println!("{:?}", v.iter().rev().enumerate().filter(|(i, _)| i % 3 == 0).map(|(_, x)| *x).collect::<Vec<_>>());
    println!("{:?} {:?}", v.rsplitn(2, |x| *x == 0).collect::<Vec<_>>(), v.iter().rev().skip_while(|x| **x != 0).count());
    println!("{:?}", [1, 2, 3].repeat(2));
    println!("{} {} {:?}", v.contains(&9), v.iter().any(|&x| x > 9), v.iter().copied().reduce(u8::max));
    let mut chars: Vec<char> = "hello".chars().collect();
    chars.rotate_left(2);
    chars.retain(|c| *c != 'l');
    println!("{}", chars.iter().collect::<String>());
}
