fn wrap(text: &str, width: usize) -> Vec<String> {
    let mut lines = vec![];
    let mut cur = String::new();
    for word in text.split_whitespace() {
        let wl = word.chars().count();
        let cl = cur.chars().count();
        if cl > 0 && cl + 1 + wl > width {
            lines.push(std::mem::take(&mut cur));
        }
        if wl > width {
            let chars: Vec<char> = word.chars().collect();
            for chunk in chars.chunks(width) {
                if !cur.is_empty() { lines.push(std::mem::take(&mut cur)); }
                cur = chunk.iter().collect();
            }
            continue;
        }
        if !cur.is_empty() { cur.push(' '); }
        cur.push_str(word);
    }
    if !cur.is_empty() { lines.push(cur); }
    lines
}

fn justify(line: &str, width: usize) -> String {
    let words: Vec<&str> = line.split(' ').collect();
    if words.len() == 1 { return format!("{:<width$}", line, width = width); }
    let total: usize = words.iter().map(|w| w.chars().count()).sum();
    let gaps = words.len() - 1;
    let spaces = width - total;
    let (base, extra) = (spaces / gaps, spaces % gaps);
    let mut out = String::new();
    for (i, w) in words.iter().enumerate() {
        out += w;
        if i < gaps { out += &" ".repeat(base + if i < extra { 1 } else { 0 }); }
    }
    out
}

fn boxed(lines: &[String], width: usize, title: &str) -> String {
    let mut s = format!("+{:-^w$}+\n", format!(" {} ", title), w = width + 2);
    for l in lines { s += &format!("| {:<w$} |\n", l, w = width); }
    s += &format!("+{}+", "-".repeat(width + 2));
    s
}

fn table(rows: &[(&str, f64, i64)]) -> String {
    let w0 = rows.iter().map(|r| r.0.chars().count()).max().unwrap_or(0).max(4);
    let mut s = format!("{:<w0$} | {:>10} | {:>8}\n", "name", "value", "count", w0 = w0);
    s += &format!("{}-+-{}-+-{}\n", "-".repeat(w0), "-".repeat(10), "-".repeat(8));
    for (n, v, c) in rows { s += &format!("{:<w0$} | {:>10.3} | {:>+8}\n", n, v, c, w0 = w0); }
    let tv: f64 = rows.iter().map(|r| r.1).sum();
    let tc: i64 = rows.iter().map(|r| r.2).sum();
    s += &format!("{:>w0$} | {:>10.3e} | {:>8}", "total", tv, tc, w0 = w0);
    s
}

fn main() {
    let text = "The quick brown fox jumps over the lazy dog. Pack my box with five dozen liquor jugs! Supercalifragilisticexpialidocious words get split. Ünïcödé wörds cöunt by chars.";
    for w in [16, 24, 40] {
        let lines = wrap(text, w);
        println!("{}", boxed(&lines, w, &format!("wrap {}", w)));
        let just: Vec<String> = lines.iter().enumerate().map(|(i, l)| if i + 1 == lines.len() { format!("{:<w$}", l, w = w) } else { justify(l, w) }).collect();
        println!("{}", boxed(&just, w, "justified"));
    }
    let centered: Vec<String> = wrap(text, 30).iter().map(|l| format!("{:^30}", l)).collect();
    println!("{}", centered.join("\n").trim_end());
    let rows = [("alpha", 1.5, 10), ("βeta", -22.125, -3), ("gamma-ray", 1234.5678, 0), ("δ", 0.0004, 999999)];
    println!("{}", table(&rows));
    let s = "  indented\n\ttabbed\r\nwindows line\n\nlast";
    for (i, l) in s.lines().enumerate() { println!("{:02}: [{}] trimmed=[{}] len={}", i, l, l.trim(), l.len()); }
    let csv = "name,age,city\n\"Smith, J\",42,NYC\nDoe,,\"San \"\"Fran\"\"\"\n";
    for line in csv.lines() {
        let mut fields = vec![]; let mut cur = String::new(); let mut inq = false; let mut chars = line.chars().peekable();
        while let Some(c) = chars.next() {
            match (c, inq) {
                ('"', true) if chars.peek() == Some(&'"') => { cur.push('"'); chars.next(); }
                ('"', _) => inq = !inq,
                (',', false) => fields.push(std::mem::take(&mut cur)),
                _ => cur.push(c),
            }
        }
        fields.push(cur);
        println!("{:?}", fields);
    }
}
