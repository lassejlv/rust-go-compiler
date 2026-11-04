// Markdown subset -> HTML converter: block parsing (headings, lists, nested
// lists, code fences, blockquotes, tables, hr, paragraphs) and an inline
// parser (emphasis, code spans, links, images, autolinks, escapes), plus a
// table of contents and slug generation.
use std::collections::HashMap;
use std::fmt::Write;
use std::iter::Peekable;
use std::str::CharIndices;

#[derive(Debug, Clone, PartialEq)]
enum Inline {
    Text(String),
    Code(String),
    Em(Vec<Inline>),
    Strong(Vec<Inline>),
    Strike(Vec<Inline>),
    Link { text: Vec<Inline>, href: String, title: Option<String> },
    Image { alt: String, src: String },
    LineBreak,
}

#[derive(Debug, Clone, PartialEq)]
enum Align {
    Left,
    Center,
    Right,
    None,
}

#[derive(Debug, Clone, PartialEq)]
enum Block {
    Heading { level: u8, content: Vec<Inline>, id: String },
    Para(Vec<Inline>),
    Code { lang: Option<String>, body: String },
    Quote(Vec<Block>),
    List { ordered: Option<u32>, items: Vec<Vec<Block>>, tight: bool },
    Table { head: Vec<Vec<Inline>>, align: Vec<Align>, rows: Vec<Vec<Vec<Inline>>> },
    Rule,
}

fn escape_html(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '&' => out.push_str("&amp;"),
            '"' => out.push_str("&quot;"),
            _ => out.push(c),
        }
    }
    out
}

struct InlineParser<'a> {
    src: &'a str,
    it: Peekable<CharIndices<'a>>,
}

impl<'a> InlineParser<'a> {
    fn new(src: &'a str) -> Self {
        InlineParser { src, it: src.char_indices().peekable() }
    }

    fn rest(&mut self) -> &'a str {
        match self.it.peek() {
            Some(&(i, _)) => &self.src[i..],
            None => "",
        }
    }

    fn skip_bytes(&mut self, n: usize) {
        let target = self.it.peek().map(|&(i, _)| i + n);
        while let Some(&(i, _)) = self.it.peek() {
            if Some(i) >= target {
                break;
            }
            self.it.next();
        }
    }

    fn parse(mut self) -> Vec<Inline> {
        self.until(None)
    }

    fn flush(text: &mut String, out: &mut Vec<Inline>) {
        if !text.is_empty() {
            out.push(Inline::Text(std::mem::take(text)));
        }
    }

    fn until(&mut self, close: Option<&str>) -> Vec<Inline> {
        let mut out = Vec::new();
        let mut text = String::new();
        loop {
            if let Some(c) = close {
                if self.rest().starts_with(c) {
                    self.skip_bytes(c.len());
                    break;
                }
            }
            let Some(&(_, ch)) = self.it.peek() else { break };
            let rest = self.rest();
            match ch {
                '\\' if rest.len() > 1 && rest[1..].starts_with(|c: char| "\\`*_[]()#+-.!~<>".contains(c)) => {
                    self.it.next();
                    text.push(self.it.next().unwrap().1);
                }
                '`' => {
                    let ticks = rest.chars().take_while(|&c| c == '`').count();
                    let fence = &rest[..ticks];
                    if let Some(end) = rest[ticks..].find(fence) {
                        Self::flush(&mut text, &mut out);
                        let code = rest[ticks..ticks + end].trim();
                        out.push(Inline::Code(code.to_string()));
                        self.skip_bytes(ticks * 2 + end);
                    } else {
                        text.push_str(fence);
                        self.skip_bytes(ticks);
                    }
                }
                '*' | '_' | '~' => {
                    let double = rest.chars().nth(1) == Some(ch);
                    let marker: String = std::iter::repeat(ch).take(if double { 2 } else { 1 }).collect();
                    if ch == '~' && !double {
                        text.push(ch);
                        self.it.next();
                        continue;
                    }
                    if rest[marker.len()..].contains(marker.as_str()) && !rest[marker.len()..].starts_with(' ') {
                        Self::flush(&mut text, &mut out);
                        self.skip_bytes(marker.len());
                        let inner = self.until(Some(&marker));
                        out.push(match (ch, double) {
                            ('~', _) => Inline::Strike(inner),
                            (_, true) => Inline::Strong(inner),
                            (_, false) => Inline::Em(inner),
                        });
                    } else {
                        text.push_str(&marker);
                        self.skip_bytes(marker.len());
                    }
                }
                '!' if rest.starts_with("![") => {
                    if let Some((alt, src, _, used)) = parse_link_parts(&rest[1..]) {
                        Self::flush(&mut text, &mut out);
                        out.push(Inline::Image { alt: alt.to_string(), src: src.to_string() });
                        self.skip_bytes(used + 1);
                    } else {
                        text.push('!');
                        self.it.next();
                    }
                }
                '[' => {
                    if let Some((label, href, title, used)) = parse_link_parts(rest) {
                        Self::flush(&mut text, &mut out);
                        let inner = InlineParser::new(label).parse();
                        out.push(Inline::Link { text: inner, href: href.to_string(), title: title.map(str::to_string) });
                        self.skip_bytes(used);
                    } else {
                        text.push('[');
                        self.it.next();
                    }
                }
                '<' if rest.starts_with("<http") => match rest.find('>') {
                    Some(end) => {
                        Self::flush(&mut text, &mut out);
                        let url = &rest[1..end];
                        out.push(Inline::Link { text: vec![Inline::Text(url.to_string())], href: url.to_string(), title: None });
                        self.skip_bytes(end + 1);
                    }
                    None => {
                        text.push('<');
                        self.it.next();
                    }
                },
                ' ' if rest.starts_with("  \n") => {
                    Self::flush(&mut text, &mut out);
                    out.push(Inline::LineBreak);
                    self.skip_bytes(3);
                }
                _ => {
                    text.push(ch);
                    self.it.next();
                }
            }
        }
        Self::flush(&mut text, &mut out);
        out
    }
}

/// `[label](href "title")` -> (label, href, title, bytes consumed)
fn parse_link_parts(s: &str) -> Option<(&str, &str, Option<&str>, usize)> {
    debug_assert!(s.starts_with('['));
    let mut depth = 0;
    let mut close = None;
    for (i, c) in s.char_indices() {
        match c {
            '[' => depth += 1,
            ']' => {
                depth -= 1;
                if depth == 0 {
                    close = Some(i);
                    break;
                }
            }
            _ => {}
        }
    }
    let close = close?;
    let after = &s[close + 1..];
    if !after.starts_with('(') {
        return None;
    }
    let end = after.find(')')?;
    let inside = after[1..end].trim();
    let (href, title) = match inside.split_once(' ') {
        Some((h, t)) => (h, Some(t.trim().trim_matches('"'))),
        None => (inside, None),
    };
    Some((&s[1..close], href, title, close + 1 + end + 1))
}

fn plain_text(inl: &[Inline]) -> String {
    inl.iter()
        .map(|i| match i {
            Inline::Text(t) | Inline::Code(t) => t.clone(),
            Inline::Em(v) | Inline::Strong(v) | Inline::Strike(v) => plain_text(v),
            Inline::Link { text, .. } => plain_text(text),
            Inline::Image { alt, .. } => alt.clone(),
            Inline::LineBreak => " ".into(),
        })
        .collect()
}

fn slugify(s: &str, used: &mut HashMap<String, usize>) -> String {
    let mut slug = String::new();
    let mut dash = false;
    for c in s.chars().flat_map(char::to_lowercase) {
        if c.is_alphanumeric() {
            slug.push(c);
            dash = false;
        } else if !dash && !slug.is_empty() {
            slug.push('-');
            dash = true;
        }
    }
    while slug.ends_with('-') {
        slug.pop();
    }
    let n = used.entry(slug.clone()).or_insert(0);
    *n += 1;
    if *n > 1 {
        format!("{}-{}", slug, *n - 1)
    } else {
        slug
    }
}

struct BlockParser<'a> {
    lines: Vec<&'a str>,
    pos: usize,
    slugs: HashMap<String, usize>,
}

fn list_marker(line: &str) -> Option<(Option<u32>, usize)> {
    let t = line.trim_start();
    let indent = line.len() - t.len();
    if let Some(rest) = t.strip_prefix("- ").or_else(|| t.strip_prefix("* ")).or_else(|| t.strip_prefix("+ ")) {
        let _ = rest;
        return Some((None, indent + 2));
    }
    let digits: String = t.chars().take_while(char::is_ascii_digit).collect();
    if !digits.is_empty() && t[digits.len()..].starts_with(". ") {
        return Some((digits.parse().ok(), indent + digits.len() + 2));
    }
    None
}

fn is_rule(l: &str) -> bool {
    let t: String = l.chars().filter(|c| !c.is_whitespace()).collect();
    t.len() >= 3 && (t.chars().all(|c| c == '-') || t.chars().all(|c| c == '*') || t.chars().all(|c| c == '_'))
}

fn split_row(l: &str) -> Vec<&str> {
    let t = l.trim().trim_start_matches('|').trim_end_matches('|');
    t.split('|').map(str::trim).collect()
}

impl<'a> BlockParser<'a> {
    fn new(src: &'a str) -> Self {
        BlockParser { lines: src.lines().collect(), pos: 0, slugs: HashMap::new() }
    }

    fn parse_all(&mut self) -> Vec<Block> {
        let mut out = Vec::new();
        while let Some(b) = self.next_block() {
            out.push(b);
        }
        out
    }

    fn next_block(&mut self) -> Option<Block> {
        while self.pos < self.lines.len() && self.lines[self.pos].trim().is_empty() {
            self.pos += 1;
        }
        let line = *self.lines.get(self.pos)?;
        let trimmed = line.trim_start();
        if let Some(rest) = trimmed.strip_prefix("```") {
            self.pos += 1;
            let lang = Some(rest.trim().to_string()).filter(|s| !s.is_empty());
            let mut body = String::new();
            while self.pos < self.lines.len() && !self.lines[self.pos].trim_start().starts_with("```") {
                body.push_str(self.lines[self.pos]);
                body.push('\n');
                self.pos += 1;
            }
            self.pos += 1; // closing fence (or eof)
            return Some(Block::Code { lang, body });
        }
        let hashes = trimmed.bytes().take_while(|&b| b == b'#').count();
        if (1..=6).contains(&hashes) && trimmed[hashes..].starts_with(' ') {
            self.pos += 1;
            let text = trimmed[hashes..].trim().trim_end_matches('#').trim_end();
            let content = InlineParser::new(text).parse();
            let id = slugify(&plain_text(&content), &mut self.slugs);
            return Some(Block::Heading { level: hashes as u8, content, id });
        }
        if is_rule(line) {
            self.pos += 1;
            return Some(Block::Rule);
        }
        if trimmed.starts_with('>') {
            let mut inner = String::new();
            while self.pos < self.lines.len() && self.lines[self.pos].trim_start().starts_with('>') {
                let l = self.lines[self.pos].trim_start()[1..].strip_prefix(' ').unwrap_or(&self.lines[self.pos].trim_start()[1..]);
                inner.push_str(l);
                inner.push('\n');
                self.pos += 1;
            }
            let mut sub = BlockParser::new(&inner);
            std::mem::swap(&mut sub.slugs, &mut self.slugs);
            let blocks = sub.parse_all();
            std::mem::swap(&mut sub.slugs, &mut self.slugs);
            return Some(Block::Quote(blocks));
        }
        if let Some((ordered, _)) = list_marker(line) {
            return Some(self.list(ordered, line.len() - trimmed.len()));
        }
        if trimmed.starts_with('|') && self.lines.get(self.pos + 1).map_or(false, |l| l.contains("---")) {
            let head = split_row(line).into_iter().map(|c| InlineParser::new(c).parse()).collect();
            let align = split_row(self.lines[self.pos + 1])
                .into_iter()
                .map(|c| match (c.starts_with(':'), c.ends_with(':')) {
                    (true, true) => Align::Center,
                    (true, false) => Align::Left,
                    (false, true) => Align::Right,
                    _ => Align::None,
                })
                .collect();
            self.pos += 2;
            let mut rows = vec![];
            while self.pos < self.lines.len() && self.lines[self.pos].trim_start().starts_with('|') {
                rows.push(split_row(self.lines[self.pos]).into_iter().map(|c| InlineParser::new(c).parse()).collect());
                self.pos += 1;
            }
            return Some(Block::Table { head, align, rows });
        }
        // paragraph: until blank line or another block start
        let mut text = String::new();
        while self.pos < self.lines.len() {
            let l = self.lines[self.pos];
            if l.trim().is_empty() || (!text.is_empty() && (l.trim_start().starts_with('#') || l.trim_start().starts_with("```") || is_rule(l) || list_marker(l).is_some())) {
                break;
            }
            if !text.is_empty() {
                text.push('\n');
            }
            text.push_str(if l.ends_with("  ") { l.trim_start() } else { l.trim() });
            self.pos += 1;
        }
        Some(Block::Para(InlineParser::new(&text).parse()))
    }

    fn list(&mut self, ordered: Option<u32>, indent: usize) -> Block {
        let mut items: Vec<Vec<Block>> = Vec::new();
        let mut tight = true;
        let mut saw_blank = false;
        while self.pos < self.lines.len() {
            let line = self.lines[self.pos];
            if line.trim().is_empty() {
                saw_blank = true;
                self.pos += 1;
                continue;
            }
            let cur_indent = line.len() - line.trim_start().len();
            match list_marker(line) {
                Some((o, content_at)) if cur_indent == indent && o.is_some() == ordered.is_some() => {
                    if saw_blank && !items.is_empty() {
                        tight = false;
                    }
                    saw_blank = false;
                    // gather this item's lines: first line + following more-indented lines
                    let mut buf = String::from(&line[content_at..]);
                    buf.push('\n');
                    self.pos += 1;
                    while self.pos < self.lines.len() {
                        let l = self.lines[self.pos];
                        let ind = l.len() - l.trim_start().len();
                        if !l.trim().is_empty() && ind < content_at {
                            break;
                        }
                        if l.trim().is_empty() {
                            // blank inside item only if followed by more indented content
                            match self.lines.get(self.pos + 1) {
                                Some(n) if n.len() - n.trim_start().len() >= content_at && !n.trim().is_empty() => {}
                                _ => break,
                            }
                        }
                        buf.push_str(l.get(content_at..).unwrap_or(""));
                        buf.push('\n');
                        self.pos += 1;
                    }
                    let mut sub = BlockParser::new(&buf);
                    std::mem::swap(&mut sub.slugs, &mut self.slugs);
                    let blocks = sub.parse_all();
                    std::mem::swap(&mut sub.slugs, &mut self.slugs);
                    items.push(blocks);
                }
                _ => break,
            }
        }
        Block::List { ordered, items, tight }
    }
}

fn render_inline(inl: &[Inline], out: &mut String) {
    for i in inl {
        match i {
            Inline::Text(t) => out.push_str(&escape_html(t)),
            Inline::Code(c) => write!(out, "<code>{}</code>", escape_html(c)).unwrap(),
            Inline::Em(v) => {
                out.push_str("<em>");
                render_inline(v, out);
                out.push_str("</em>");
            }
            Inline::Strong(v) => {
                out.push_str("<strong>");
                render_inline(v, out);
                out.push_str("</strong>");
            }
            Inline::Strike(v) => {
                out.push_str("<del>");
                render_inline(v, out);
                out.push_str("</del>");
            }
            Inline::Link { text, href, title } => {
                write!(out, "<a href=\"{}\"", escape_html(href)).unwrap();
                if let Some(t) = title {
                    write!(out, " title=\"{}\"", escape_html(t)).unwrap();
                }
                out.push('>');
                render_inline(text, out);
                out.push_str("</a>");
            }
            Inline::Image { alt, src } => write!(out, "<img src=\"{}\" alt=\"{}\">", escape_html(src), escape_html(alt)).unwrap(),
            Inline::LineBreak => out.push_str("<br>\n"),
        }
    }
}

fn render(blocks: &[Block], out: &mut String, tight: bool) {
    for b in blocks {
        match b {
            Block::Heading { level, content, id } => {
                write!(out, "<h{} id=\"{}\">", level, id).unwrap();
                render_inline(content, out);
                writeln!(out, "</h{}>", level).unwrap();
            }
            Block::Para(inl) => {
                if !tight {
                    out.push_str("<p>");
                }
                render_inline(inl, out);
                if !tight {
                    out.push_str("</p>");
                }
                out.push('\n');
            }
            Block::Code { lang, body } => {
                match lang {
                    Some(l) => write!(out, "<pre><code class=\"language-{}\">", l).unwrap(),
                    None => out.push_str("<pre><code>"),
                }
                out.push_str(&escape_html(body));
                out.push_str("</code></pre>\n");
            }
            Block::Quote(inner) => {
                out.push_str("<blockquote>\n");
                render(inner, out, false);
                out.push_str("</blockquote>\n");
            }
            Block::List { ordered, items, tight } => {
                match ordered {
                    Some(1) => out.push_str("<ol>\n"),
                    Some(n) => writeln!(out, "<ol start=\"{}\">", n).unwrap(),
                    None => out.push_str("<ul>\n"),
                }
                for it in items {
                    out.push_str("<li>");
                    let mut inner = String::new();
                    render(it, &mut inner, *tight);
                    out.push_str(inner.trim_end_matches('\n'));
                    out.push_str("</li>\n");
                }
                out.push_str(if ordered.is_some() { "</ol>\n" } else { "</ul>\n" });
            }
            Block::Table { head, align, rows } => {
                out.push_str("<table>\n<thead>\n<tr>");
                let attr = |i: usize| match align.get(i).unwrap_or(&Align::None) {
                    Align::None => String::new(),
                    a => format!(" align=\"{}\"", format!("{:?}", a).to_lowercase()),
                };
                for (i, h) in head.iter().enumerate() {
                    write!(out, "<th{}>", attr(i)).unwrap();
                    render_inline(h, out);
                    out.push_str("</th>");
                }
                out.push_str("</tr>\n</thead>\n<tbody>\n");
                for r in rows {
                    out.push_str("<tr>");
                    for (i, c) in r.iter().enumerate() {
                        write!(out, "<td{}>", attr(i)).unwrap();
                        render_inline(c, out);
                        out.push_str("</td>");
                    }
                    out.push_str("</tr>\n");
                }
                out.push_str("</tbody>\n</table>\n");
            }
            Block::Rule => out.push_str("<hr>\n"),
        }
    }
}

fn toc(blocks: &[Block]) -> Vec<(u8, String, String)> {
    let mut v = Vec::new();
    for b in blocks {
        match b {
            Block::Heading { level, content, id } => v.push((*level, plain_text(content), id.clone())),
            Block::Quote(inner) => v.extend(toc(inner)),
            Block::List { items, .. } => items.iter().for_each(|it| v.extend(toc(it))),
            _ => {}
        }
    }
    v
}

const DOC: &str = r#"# The *Rustgo* Manual

Welcome to **rustgo** -- a `Rust` compiler written in _Go_.
It supports ~~some~~ **most** of the language. See [the repo](https://example.com/rustgo "Rustgo home")
or <https://example.org/docs>.

## Getting started

1. Install Go
2. Build it:
   ```sh
   go build -o rustgo ./cmd/rustgo
   ```
3. Run `rustgo file.rs`

- Fast *enough*
- Written in Go
  - with **nested** lists
  - and `code`
- Escapes: \*not emphasis\* and 3 < 4 & 5 > 2

## Getting started

> **Note:** headings with the same text get unique ids.
> > nested quote with [link](/x)

| Feature | Status | Notes |
|:--------|:------:|------:|
| Traits | done | `dyn` too |
| Async | partial | *experimental* |

---

Line one with break
line two. ![logo](img/logo.png) and an unclosed *star and a [bracket.

7. seven
8. eight

* loose item one

* loose item two
"#;

fn main() {
    let blocks = BlockParser::new(DOC).parse_all();
    println!("{} top-level blocks", blocks.len());
    let kinds: Vec<&str> = blocks
        .iter()
        .map(|b| match b {
            Block::Heading { .. } => "h",
            Block::Para(_) => "p",
            Block::Code { .. } => "code",
            Block::Quote(_) => "quote",
            Block::List { .. } => "list",
            Block::Table { .. } => "table",
            Block::Rule => "hr",
        })
        .collect();
    println!("{}", kinds.join(","));
    let mut html = String::new();
    render(&blocks, &mut html, false);
    print!("{}", html);
    println!("--- toc ---");
    for (lvl, text, id) in toc(&blocks) {
        println!("{}- [{}](#{})", "  ".repeat(lvl as usize - 1), text, id);
    }
    let words = plain_text(&InlineParser::new("**a** _b_ `c` [d](e) ![f](g)").parse());
    println!("{:?}", words);
    let stats: HashMap<char, usize> = html.chars().filter(|c| "<>/".contains(*c)).fold(HashMap::new(), |mut m, c| {
        *m.entry(c).or_default() += 1;
        m
    });
    let mut stats: Vec<_> = stats.into_iter().collect();
    stats.sort();
    println!("{:?} total bytes {}", stats, html.len());
}
