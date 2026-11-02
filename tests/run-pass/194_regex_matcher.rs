// A small backtracking regex engine supporting: literals, '.', '*', '+', '?',
// character classes [a-z] / [^0-9], anchors ^ and $, groups (a|b) with alternation.

#[derive(Debug, Clone)]
enum Node {
    Char(char),
    Any,
    Class(Vec<(char, char)>, bool),
    Group(Vec<Vec<Node>>),
    Repeat(Box<Node>, usize, Option<usize>),
    Start,
    End,
}

struct Parser<'a> {
    chars: std::iter::Peekable<std::str::Chars<'a>>,
}

impl<'a> Parser<'a> {
    fn parse_alt(&mut self) -> Vec<Vec<Node>> {
        let mut alts = vec![self.parse_seq()];
        while let Some('|') = self.chars.peek() {
            self.chars.next();
            alts.push(self.parse_seq());
        }
        alts
    }
    fn parse_seq(&mut self) -> Vec<Node> {
        let mut seq = Vec::new();
        while let Some(&c) = self.chars.peek() {
            if c == '|' || c == ')' {
                break;
            }
            self.chars.next();
            let atom = match c {
                '.' => Node::Any,
                '^' => Node::Start,
                '$' => Node::End,
                '(' => {
                    let g = self.parse_alt();
                    assert_eq!(self.chars.next(), Some(')'));
                    Node::Group(g)
                }
                '[' => {
                    let mut negate = false;
                    if let Some('^') = self.chars.peek() {
                        negate = true;
                        self.chars.next();
                    }
                    let mut ranges = Vec::new();
                    while let Some(ch) = self.chars.next() {
                        if ch == ']' {
                            break;
                        }
                        if let Some('-') = self.chars.peek() {
                            self.chars.next();
                            let hi = self.chars.next().unwrap();
                            ranges.push((ch, hi));
                        } else {
                            ranges.push((ch, ch));
                        }
                    }
                    Node::Class(ranges, negate)
                }
                '\\' => Node::Char(self.chars.next().unwrap()),
                other => Node::Char(other),
            };
            let node = match self.chars.peek() {
                Some('*') => Some(Node::Repeat(Box::new(atom.clone()), 0, None)),
                Some('+') => Some(Node::Repeat(Box::new(atom.clone()), 1, None)),
                Some('?') => Some(Node::Repeat(Box::new(atom.clone()), 0, Some(1))),
                _ => None,
            };
            match node {
                Some(n) => {
                    self.chars.next();
                    seq.push(n);
                }
                None => seq.push(atom),
            }
        }
        seq
    }
}

struct Regex {
    alts: Vec<Vec<Node>>,
}

impl Regex {
    fn new(p: &str) -> Regex {
        let mut parser = Parser { chars: p.chars().peekable() };
        Regex { alts: parser.parse_alt() }
    }

    // Match a sequence of nodes starting at pos; k is the continuation.
    fn m_seq(&self, nodes: &[Node], text: &[char], pos: usize, k: &mut dyn FnMut(usize) -> bool) -> bool {
        match nodes.split_first() {
            None => k(pos),
            Some((first, rest)) => self.m_node(first, text, pos, &mut |p| self.m_seq(rest, text, p, k)),
        }
    }

    fn m_node(&self, node: &Node, text: &[char], pos: usize, k: &mut dyn FnMut(usize) -> bool) -> bool {
        match node {
            Node::Char(c) => pos < text.len() && text[pos] == *c && k(pos + 1),
            Node::Any => pos < text.len() && k(pos + 1),
            Node::Class(ranges, neg) => {
                pos < text.len() && {
                    let ch = text[pos];
                    let inside = ranges.iter().any(|&(lo, hi)| lo <= ch && ch <= hi);
                    inside != *neg
                } && k(pos + 1)
            }
            Node::Start => pos == 0 && k(pos),
            Node::End => pos == text.len() && k(pos),
            Node::Group(alts) => alts.iter().any(|seq| self.m_seq(seq, text, pos, k)),
            Node::Repeat(inner, min, max) => self.m_repeat(inner, *min, *max, 0, text, pos, k),
        }
    }

    fn m_repeat(&self, inner: &Node, min: usize, max: Option<usize>, count: usize, text: &[char], pos: usize, k: &mut dyn FnMut(usize) -> bool) -> bool {
        // greedy: try one more first
        if max.map_or(true, |m| count < m) {
            let more = self.m_node(inner, text, pos, &mut |p| p != pos && self.m_repeat(inner, min, max, count + 1, text, p, k));
            if more {
                return true;
            }
        }
        count >= min && k(pos)
    }

    fn find(&self, s: &str) -> Option<(usize, usize)> {
        let text: Vec<char> = s.chars().collect();
        for start in 0..=text.len() {
            let mut end = None;
            let found = self.alts.iter().any(|seq| {
                self.m_seq(seq, &text, start, &mut |p| {
                    end = Some(p);
                    true
                })
            });
            if found {
                return Some((start, end.unwrap()));
            }
        }
        None
    }

    fn is_match(&self, s: &str) -> bool {
        self.find(s).is_some()
    }
}

fn main() {
    let cases: &[(&str, &[&str])] = &[
        ("abc", &["abc", "xabcx", "ab"]),
        ("^a.c$", &["abc", "aXc", "abcd", "ac"]),
        ("ab*c", &["ac", "abc", "abbbbc", "adc"]),
        ("ab+c", &["ac", "abc", "abbbc"]),
        ("colou?r", &["color", "colour", "colouur"]),
        ("[0-9]+", &["abc123def", "no digits", "7"]),
        ("^[A-Z][a-z]*$", &["Hello", "hello", "HeLLo", "X"]),
        ("[^aeiou ]+", &["aei", "rhythm", "a b"]),
        ("(cat|dog)s?$", &["cats", "dog", "cow", "hotdogs"]),
        ("^(a|b)*abb$", &["abb", "aababb", "abab", "babb"]),
        ("a.*b", &["axxxbyyb", "ab", "ba"]),
        ("\\.txt$", &["notes.txt", "notesxtxt", "a.txt.bak"]),
        ("^$", &["", "x"]),
    ];
    for (pat, inputs) in cases {
        let re = Regex::new(pat);
        let results: Vec<String> = inputs
            .iter()
            .map(|s| match re.find(s) {
                Some((a, b)) => format!("{:?}=[{}..{}]", s, a, b),
                None => format!("{:?}=no", s),
            })
            .collect();
        println!("{:<16} {}", pat, results.join(" "));
    }
    let email = Regex::new("^[a-z0-9.]+@[a-z]+(\\.[a-z]+)+$");
    for e in ["bob@example.com", "a.b@c.co.uk", "bad@", "@x.com", "x@y"] {
        println!("{:<16} email? {}", e, email.is_match(e));
    }
}
