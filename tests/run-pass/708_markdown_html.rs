// A small Markdown-to-HTML converter: headings, paragraphs, lists (ordered
// and unordered), code blocks, block quotes, inline emphasis/code/links.
use std::fmt::Write;

#[derive(Debug, PartialEq)]
enum Block {
    Heading(usize, String),
    Para(Vec<String>),
    List { ordered: bool, items: Vec<String> },
    Code(Option<String>, Vec<String>),
    Quote(Vec<String>),
    Rule,
}

fn escape_html(s: &str) -> String {
    s.chars().fold(String::new(), |mut acc, c| {
        match c {
            '<' => acc.push_str("&lt;"),
            '>' => acc.push_str("&gt;"),
            '&' => acc.push_str("&amp;"),
            '"' => acc.push_str("&quot;"),
            _ => acc.push(c),
        }
        acc
    })
}

fn inline(s: &str) -> String {
    let chars: Vec<char> = s.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    let find = |from: usize, pat: &[char]| -> Option<usize> {
        (from..chars.len()).find(|&j| chars[j..].starts_with(pat))
    };
    while i < chars.len() {
        let c = chars[i];
        if c == '`' {
            if let Some(end) = find(i + 1, &['`']) {
                let code: String = chars[i + 1..end].iter().collect();
                write!(out, "<code>{}</code>", escape_html(&code)).unwrap();
                i = end + 1;
                continue;
            }
        }
        if c == '*' && chars.get(i + 1) == Some(&'*') {
            if let Some(end) = find(i + 2, &['*', '*']) {
                let inner: String = chars[i + 2..end].iter().collect();
                write!(out, "<strong>{}</strong>", inline(&inner)).unwrap();
                i = end + 2;
                continue;
            }
        }
        if c == '*' || c == '_' {
            if let Some(end) = find(i + 1, &[c]) {
                if end > i + 1 {
                    let inner: String = chars[i + 1..end].iter().collect();
                    write!(out, "<em>{}</em>", inline(&inner)).unwrap();
                    i = end + 1;
                    continue;
                }
            }
        }
        if c == '[' {
            if let Some(mid) = find(i + 1, &[']', '(']) {
                if let Some(end) = find(mid + 2, &[')']) {
                    let text: String = chars[i + 1..mid].iter().collect();
                    let url: String = chars[mid + 2..end].iter().collect();
                    write!(out, "<a href=\"{}\">{}</a>", escape_html(&url), inline(&text)).unwrap();
                    i = end + 1;
                    continue;
                }
            }
        }
        out.push_str(&escape_html(&c.to_string()));
        i += 1;
    }
    out
}

fn list_item(line: &str) -> Option<(bool, &str)> {
    let t = line.trim_start();
    if let Some(rest) = t.strip_prefix("- ").or_else(|| t.strip_prefix("* ")) {
        return Some((false, rest));
    }
    let digits = t.chars().take_while(|c| c.is_ascii_digit()).count();
    if digits > 0 {
        if let Some(rest) = t[digits..].strip_prefix(". ") {
            return Some((true, rest));
        }
    }
    None
}

fn parse(src: &str) -> Vec<Block> {
    let mut blocks = Vec::new();
    let mut lines = src.lines().peekable();
    while let Some(line) = lines.next() {
        let t = line.trim_end();
        if t.trim().is_empty() {
            continue;
        }
        if let Some(lang) = t.strip_prefix("```") {
            let mut body = Vec::new();
            for l in lines.by_ref() {
                if l.trim_end() == "```" { break; }
                body.push(l.to_string());
            }
            let lang = if lang.is_empty() { None } else { Some(lang.to_string()) };
            blocks.push(Block::Code(lang, body));
        } else if t.starts_with('#') {
            let level = t.chars().take_while(|&c| c == '#').count();
            blocks.push(Block::Heading(level.min(6), t[level..].trim().to_string()));
        } else if t == "---" || t == "***" {
            blocks.push(Block::Rule);
        } else if t.starts_with('>') {
            let mut q = vec![t[1..].trim().to_string()];
            while let Some(n) = lines.peek() {
                if let Some(r) = n.strip_prefix('>') { q.push(r.trim().to_string()); lines.next(); } else { break; }
            }
            blocks.push(Block::Quote(q));
        } else if let Some((ordered, first)) = list_item(t) {
            let mut items = vec![first.to_string()];
            while let Some(n) = lines.peek() {
                match list_item(n) {
                    Some((o, rest)) if o == ordered => { items.push(rest.to_string()); lines.next(); }
                    _ => break,
                }
            }
            blocks.push(Block::List { ordered, items });
        } else {
            let mut para = vec![t.trim().to_string()];
            while let Some(n) = lines.peek() {
                let n = n.trim();
                if n.is_empty() || n.starts_with('#') || n.starts_with("```") || n.starts_with('>') || list_item(n).is_some() {
                    break;
                }
                para.push(n.to_string());
                lines.next();
            }
            blocks.push(Block::Para(para));
        }
    }
    blocks
}

fn render(blocks: &[Block]) -> String {
    let mut out = String::new();
    for b in blocks {
        match b {
            Block::Heading(n, t) => writeln!(out, "<h{n}>{}</h{n}>", inline(t)).unwrap(),
            Block::Para(ls) => writeln!(out, "<p>{}</p>", inline(&ls.join(" "))).unwrap(),
            Block::List { ordered, items } => {
                let tag = if *ordered { "ol" } else { "ul" };
                writeln!(out, "<{}>", tag).unwrap();
                for it in items {
                    writeln!(out, "  <li>{}</li>", inline(it)).unwrap();
                }
                writeln!(out, "</{}>", tag).unwrap();
            }
            Block::Code(lang, body) => {
                match lang {
                    Some(l) => write!(out, "<pre><code class=\"language-{}\">", l).unwrap(),
                    None => out.push_str("<pre><code>"),
                }
                out.push_str(&escape_html(&body.join("\n")));
                out.push_str("</code></pre>\n");
            }
            Block::Quote(ls) => writeln!(out, "<blockquote>{}</blockquote>", inline(&ls.join(" "))).unwrap(),
            Block::Rule => out.push_str("<hr/>\n"),
        }
    }
    out
}

const DOC: &str = r#"# Rust *and* Go

This is a **bold** claim with `code <tags>` and a [link](http://x.y/?a=1&b=2).
It continues on the _second_ line.

## Lists
- first item
- second **item**
* third with `x*y`
1. one
2. two
10. ten

> quoted text
> more *quote*

```rust
fn main() { println!("<hi>"); }
```

---
Trailing para with a lone * star and [broken link.
### Done
"#;

fn main() {
    let blocks = parse(DOC);
    println!("{} blocks", blocks.len());
    for b in &blocks {
        let kind = match b {
            Block::Heading(n, _) => format!("H{}", n),
            Block::Para(l) => format!("P({})", l.len()),
            Block::List { ordered: true, items } => format!("OL({})", items.len()),
            Block::List { ordered: false, items } => format!("UL({})", items.len()),
            Block::Code(Some(l), b) => format!("CODE[{}]({})", l, b.len()),
            Block::Code(None, b) => format!("CODE({})", b.len()),
            Block::Quote(_) => "QUOTE".to_string(),
            Block::Rule => "HR".to_string(),
        };
        print!("{} ", kind);
    }
    println!();
    print!("{}", render(&blocks));
    println!("{:?}", parse("```\nraw\n"));
    println!("{}", inline("**_nested_ strong** and *a*b*"));
}
