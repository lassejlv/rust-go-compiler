// A markdown -> HTML converter: block parsing (headings, paragraphs, lists,
// nested lists, code fences, blockquotes, tables, hr) and inline parsing
// (emphasis, strong, code spans, links, images, autolinks, escapes).
use std::collections::HashMap;
use std::fmt::Write;

#[derive(Debug, PartialEq)]
enum Block {
    Heading(usize, String),
    Para(Vec<String>),
    Code(Option<String>, Vec<String>),
    Quote(Vec<Block>),
    List { ordered: bool, start: u32, items: Vec<Vec<Block>> },
    Table { header: Vec<String>, align: Vec<Align>, rows: Vec<Vec<String>> },
    Rule,
}

#[derive(Debug, PartialEq, Clone, Copy)]
enum Align { None, Left, Center, Right }

fn escape_html(s: &str) -> String {
    let mut o = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '<' => o.push_str("&lt;"),
            '>' => o.push_str("&gt;"),
            '&' => o.push_str("&amp;"),
            '"' => o.push_str("&quot;"),
            c => o.push(c),
        }
    }
    o
}

fn list_marker(line: &str) -> Option<(bool, u32, usize)> {
    let t = line.trim_start();
    let indent = line.len() - t.len();
    if let Some(rest) = t.strip_prefix("- ").or_else(|| t.strip_prefix("* ")).or_else(|| t.strip_prefix("+ ")) {
        let _ = rest;
        return Some((false, 1, indent + 2));
    }
    let digits: String = t.chars().take_while(|c| c.is_ascii_digit()).collect();
    if !digits.is_empty() && digits.len() < 10 {
        let after = &t[digits.len()..];
        if after.starts_with(". ") || after.starts_with(") ") {
            return Some((true, digits.parse().unwrap(), indent + digits.len() + 2));
        }
    }
    None
}

fn is_rule(line: &str) -> bool {
    let t: String = line.chars().filter(|c| !c.is_whitespace()).collect();
    t.len() >= 3 && (t.chars().all(|c| c == '-') || t.chars().all(|c| c == '*') || t.chars().all(|c| c == '_'))
}

fn split_row(line: &str) -> Vec<String> {
    let t = line.trim().trim_start_matches('|').trim_end_matches('|');
    t.split('|').map(|c| c.trim().to_string()).collect()
}

fn parse_blocks(lines: &[&str]) -> Vec<Block> {
    let mut blocks = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];
        let t = line.trim();
        if t.is_empty() {
            i += 1;
            continue;
        }
        if let Some(fence) = t.strip_prefix("```") {
            let lang = if fence.trim().is_empty() { None } else { Some(fence.trim().to_string()) };
            let mut body = Vec::new();
            i += 1;
            while i < lines.len() && !lines[i].trim_start().starts_with("```") {
                body.push(lines[i].to_string());
                i += 1;
            }
            i += 1;
            blocks.push(Block::Code(lang, body));
            continue;
        }
        if t.starts_with('#') {
            let level = t.chars().take_while(|&c| c == '#').count();
            if level <= 6 && t[level..].starts_with(' ') {
                blocks.push(Block::Heading(level, t[level..].trim().trim_end_matches('#').trim_end().to_string()));
                i += 1;
                continue;
            }
        }
        if is_rule(t) && list_marker(line).map_or(true, |_| t.chars().filter(|c| *c != ' ').count() >= 3 && !t.contains(|c: char| c.is_alphanumeric())) {
            blocks.push(Block::Rule);
            i += 1;
            continue;
        }
        if t.starts_with('>') {
            let mut inner = Vec::new();
            while i < lines.len() && lines[i].trim_start().starts_with('>') {
                let l = lines[i].trim_start()[1..].strip_prefix(' ').unwrap_or(&lines[i].trim_start()[1..]);
                inner.push(l);
                i += 1;
            }
            blocks.push(Block::Quote(parse_blocks(&inner)));
            continue;
        }
        if t.starts_with('|') && i + 1 < lines.len() && lines[i + 1].trim().starts_with('|') && lines[i + 1].contains("---") {
            let header = split_row(t);
            let align = split_row(lines[i + 1])
                .iter()
                .map(|c| match (c.starts_with(':'), c.ends_with(':')) {
                    (true, true) => Align::Center,
                    (true, false) => Align::Left,
                    (false, true) => Align::Right,
                    _ => Align::None,
                })
                .collect();
            i += 2;
            let mut rows = Vec::new();
            while i < lines.len() && lines[i].trim().starts_with('|') {
                rows.push(split_row(lines[i]));
                i += 1;
            }
            blocks.push(Block::Table { header, align, rows });
            continue;
        }
        if let Some((ordered, start, _)) = list_marker(line) {
            let base_indent = line.len() - line.trim_start().len();
            let mut items: Vec<Vec<&str>> = Vec::new();
            while i < lines.len() {
                let l = lines[i];
                let ind = l.len() - l.trim_start().len();
                if l.trim().is_empty() {
                    if i + 1 < lines.len() && (lines[i + 1].len() - lines[i + 1].trim_start().len() > base_indent || list_marker(lines[i + 1]).map_or(false, |m| m.0 == ordered)) {
                        i += 1;
                        continue;
                    }
                    break;
                }
                match list_marker(l) {
                    Some((o, _, content)) if ind == base_indent && o == ordered => {
                        items.push(vec![&l[content..]]);
                    }
                    _ if ind > base_indent && !items.is_empty() => {
                        let strip = (base_indent + 2).min(ind);
                        items.last_mut().unwrap().push(&l[strip..]);
                    }
                    _ if ind == base_indent && list_marker(l).is_none() && !items.is_empty() => {
                        // lazy continuation
                        items.last_mut().unwrap().push(l.trim_start());
                    }
                    _ => break,
                }
                i += 1;
            }
            let items = items.iter().map(|it| parse_blocks(it)).collect();
            blocks.push(Block::List { ordered, start, items });
            continue;
        }
        // paragraph
        let mut para = Vec::new();
        while i < lines.len() {
            let l = lines[i].trim();
            if !para.is_empty() && (l.is_empty() || l.starts_with('#') || l.starts_with("```") || l.starts_with('>') || (is_rule(l) && para.is_empty()) || list_marker(lines[i]).is_some()) {
                break;
            }
            para.push(l.to_string());
            i += 1;
        }
        blocks.push(Block::Para(para));
    }
    blocks
}

fn inline(s: &str, refs: &HashMap<String, String>) -> String {
    let cs: Vec<char> = s.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    let find = |from: usize, pat: &[char]| -> Option<usize> {
        (from..cs.len()).find(|&j| cs[j..].starts_with(pat))
    };
    while i < cs.len() {
        let c = cs[i];
        match c {
            '\\' if i + 1 < cs.len() && "\\`*_[]()#+-.!<>".contains(cs[i + 1]) => {
                out.push_str(&escape_html(&cs[i + 1].to_string()));
                i += 2;
            }
            '`' => {
                let ticks = cs[i..].iter().take_while(|&&c| c == '`').count();
                let pat: Vec<char> = vec!['`'; ticks];
                match find(i + ticks, &pat) {
                    Some(end) => {
                        let code: String = cs[i + ticks..end].iter().collect();
                        write!(out, "<code>{}</code>", escape_html(code.trim())).unwrap();
                        i = end + ticks;
                    }
                    None => {
                        out.push_str(&"`".repeat(ticks));
                        i += ticks;
                    }
                }
            }
            '*' | '_' => {
                let strong = i + 1 < cs.len() && cs[i + 1] == c;
                let pat: Vec<char> = if strong { vec![c, c] } else { vec![c] };
                let start = i + pat.len();
                match find(start, &pat) {
                    Some(end) if end > start && !cs[start].is_whitespace() => {
                        let inner: String = cs[start..end].iter().collect();
                        let tag = if strong { "strong" } else { "em" };
                        write!(out, "<{}>{}</{}>", tag, inline(&inner, refs), tag).unwrap();
                        i = end + pat.len();
                    }
                    _ => {
                        out.push(c);
                        i += 1;
                    }
                }
            }
            '!' | '[' => {
                let img = c == '!';
                let open = if img { i + 1 } else { i };
                if open >= cs.len() || cs[open] != '[' {
                    out.push(c);
                    i += 1;
                    continue;
                }
                let close = find(open + 1, &[']']);
                let parsed = close.and_then(|cl| {
                    let text: String = cs[open + 1..cl].iter().collect();
                    if cs.get(cl + 1) == Some(&'(') {
                        let end = find(cl + 2, &[')'])?;
                        let target: String = cs[cl + 2..end].iter().collect();
                        let (url, title) = match target.split_once(" \"") {
                            Some((u, t)) => (u.to_string(), Some(t.trim_end_matches('"').to_string())),
                            None => (target, None),
                        };
                        Some((text, url, title, end + 1))
                    } else if cs.get(cl + 1) == Some(&'[') {
                        let end = find(cl + 2, &[']'])?;
                        let mut key: String = cs[cl + 2..end].iter().collect();
                        if key.is_empty() {
                            key = text.clone();
                        }
                        refs.get(&key.to_lowercase()).map(|u| (text, u.clone(), None, end + 1))
                    } else {
                        None
                    }
                });
                match parsed {
                    Some((text, url, title, next)) => {
                        let t = title.map(|t| format!(" title=\"{}\"", escape_html(&t))).unwrap_or_default();
                        if img {
                            write!(out, "<img src=\"{}\" alt=\"{}\"{}>", escape_html(&url), escape_html(&text), t).unwrap();
                        } else {
                            write!(out, "<a href=\"{}\"{}>{}</a>", escape_html(&url), t, inline(&text, refs)).unwrap();
                        }
                        i = next;
                    }
                    None => {
                        out.push(c);
                        i += 1;
                    }
                }
            }
            '<' => {
                let end = find(i + 1, &['>']);
                let inner: Option<String> = end.map(|e| cs[i + 1..e].iter().collect());
                match inner {
                    Some(u) if u.starts_with("http://") || u.starts_with("https://") => {
                        write!(out, "<a href=\"{0}\">{0}</a>", escape_html(&u)).unwrap();
                        i = end.unwrap() + 1;
                    }
                    _ => {
                        out.push_str("&lt;");
                        i += 1;
                    }
                }
            }
            ' ' if cs[i..].starts_with(&[' ', ' ']) && i + 2 == cs.len() => {
                out.push_str("<br>");
                i = cs.len();
            }
            c => {
                out.push_str(&escape_html(&c.to_string()));
                i += 1;
            }
        }
    }
    out
}

fn render(blocks: &[Block], refs: &HashMap<String, String>, out: &mut String, tight: bool) {
    for b in blocks {
        match b {
            Block::Heading(l, t) => {
                let id: String = t.to_lowercase().chars().filter_map(|c| if c.is_alphanumeric() { Some(c) } else if c == ' ' || c == '-' { Some('-') } else { None }).collect();
                writeln!(out, "<h{0} id=\"{2}\">{1}</h{0}>", l, inline(t, refs), id).unwrap();
            }
            Block::Para(lines) => {
                let joined: Vec<String> = lines.iter().map(|l| inline(l, refs)).collect();
                if tight {
                    out.push_str(&joined.join("\n"));
                    out.push('\n');
                } else {
                    writeln!(out, "<p>{}</p>", joined.join("\n")).unwrap();
                }
            }
            Block::Code(lang, body) => {
                match lang {
                    Some(l) => write!(out, "<pre><code class=\"language-{}\">", l).unwrap(),
                    None => out.push_str("<pre><code>"),
                }
                for l in body {
                    out.push_str(&escape_html(l));
                    out.push('\n');
                }
                out.push_str("</code></pre>\n");
            }
            Block::Quote(inner) => {
                out.push_str("<blockquote>\n");
                render(inner, refs, out, false);
                out.push_str("</blockquote>\n");
            }
            Block::List { ordered, start, items } => {
                if *ordered {
                    if *start != 1 { writeln!(out, "<ol start=\"{}\">", start).unwrap(); } else { out.push_str("<ol>\n"); }
                } else {
                    out.push_str("<ul>\n");
                }
                for it in items {
                    out.push_str("<li>");
                    let mut inner = String::new();
                    render(it, refs, &mut inner, true);
                    let inner = inner.trim_end_matches('\n');
                    if inner.contains('\n') && !inner.starts_with('<') {
                        let (first, rest) = inner.split_once('\n').unwrap();
                        write!(out, "{}\n{}\n", first, rest).unwrap();
                    } else {
                        out.push_str(inner);
                    }
                    out.push_str("</li>\n");
                }
                out.push_str(if *ordered { "</ol>\n" } else { "</ul>\n" });
            }
            Block::Table { header, align, rows } => {
                out.push_str("<table>\n<thead>\n<tr>");
                let attr = |k: usize| match align.get(k).copied().unwrap_or(Align::None) {
                    Align::None => String::new(),
                    Align::Left => " align=\"left\"".into(),
                    Align::Center => " align=\"center\"".into(),
                    Align::Right => " align=\"right\"".into(),
                };
                for (k, h) in header.iter().enumerate() {
                    write!(out, "<th{}>{}</th>", attr(k), inline(h, refs)).unwrap();
                }
                out.push_str("</tr>\n</thead>\n<tbody>\n");
                for r in rows {
                    out.push_str("<tr>");
                    for k in 0..header.len() {
                        write!(out, "<td{}>{}</td>", attr(k), inline(r.get(k).map(|s| s.as_str()).unwrap_or(""), refs)).unwrap();
                    }
                    out.push_str("</tr>\n");
                }
                out.push_str("</tbody>\n</table>\n");
            }
            Block::Rule => out.push_str("<hr>\n"),
        }
    }
}

fn convert(md: &str) -> String {
    let mut refs = HashMap::new();
    let mut lines = Vec::new();
    for l in md.lines() {
        let t = l.trim();
        if t.starts_with('[') {
            if let Some((k, v)) = t[1..].split_once("]:") {
                refs.insert(k.to_lowercase(), v.trim().to_string());
                continue;
            }
        }
        lines.push(l);
    }
    let blocks = parse_blocks(&lines);
    let mut out = String::new();
    render(&blocks, &refs, &mut out, false);
    out
}

const DOC: &str = r#"# Project *Nebula* — Notes #

Nebula is a **fast**, _small_ & "friendly" tool.
It handles `a < b && c` and ``code with ` tick``.
Line with hard break
next line.

## Install

1. Download from <https://example.com/dl?x=1&y=2>
2. Run `make`
   - on *Linux*: use gcc
   - on macOS: use clang
3. Profit

5) five
6) six

* star item with [a link](http://a.b/c "Title here")
* see [the docs][docs] or [Docs][]
+ plus is still a bullet

> Quote level one
> with **bold**
>
> > nested quote
> - list in quote

---

```rust
fn main() {
    println!("<hi> & bye");
}
```

| Name | Qty | Price |
|:-----|:---:|------:|
| Apple | 3 | $1.20 |
| Kiwi \| Lime | 10 | $0.50 |
| Only |

![logo](/img/logo.png "Logo") and escaped \*not em\* and 2 * 3 * 4.
Unclosed **bold and [bracket and <tag>.

[docs]: https://docs.example.org
***
###### Six
####### Seven is a paragraph
"#;

fn main() {
    let html = convert(DOC);
    print!("{}", html);
    println!("--- stats ---");
    let tags = ["<p>", "<li>", "<h", "<a ", "<code", "<strong>", "<em>", "<td", "<blockquote>"];
    for t in tags {
        println!("{:<14} {}", t, html.matches(t).count());
    }
    println!("{}", convert("plain"));
    println!("{:?}", convert(""));
    println!("{:?}", convert("- a\n- b\n\n- c\n\nafter"));
}
