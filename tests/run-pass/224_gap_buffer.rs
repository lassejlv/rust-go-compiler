// A gap buffer text editor core with cursor movement, undo stack and line/column queries.
use std::fmt;

struct GapBuffer {
    buf: Vec<char>,
    gap_start: usize,
    gap_end: usize,
}

impl GapBuffer {
    fn new(cap: usize) -> Self {
        GapBuffer { buf: vec!['\0'; cap], gap_start: 0, gap_end: cap }
    }
    fn len(&self) -> usize {
        self.buf.len() - (self.gap_end - self.gap_start)
    }
    fn cursor(&self) -> usize {
        self.gap_start
    }
    fn gap_len(&self) -> usize {
        self.gap_end - self.gap_start
    }
    fn grow(&mut self) {
        let old_len = self.buf.len();
        let new_len = (old_len * 2).max(8);
        let extra = new_len - old_len;
        let mut nb = Vec::with_capacity(new_len);
        nb.extend_from_slice(&self.buf[..self.gap_start]);
        nb.extend(std::iter::repeat('\0').take(self.gap_len() + extra));
        nb.extend_from_slice(&self.buf[self.gap_end..]);
        self.gap_end += extra;
        self.buf = nb;
    }
    fn move_to(&mut self, pos: usize) {
        let pos = pos.min(self.len());
        while self.gap_start > pos {
            self.gap_start -= 1;
            self.gap_end -= 1;
            self.buf[self.gap_end] = self.buf[self.gap_start];
        }
        while self.gap_start < pos {
            self.buf[self.gap_start] = self.buf[self.gap_end];
            self.gap_start += 1;
            self.gap_end += 1;
        }
    }
    fn insert(&mut self, c: char) {
        if self.gap_len() == 0 {
            self.grow();
        }
        self.buf[self.gap_start] = c;
        self.gap_start += 1;
    }
    fn insert_str(&mut self, s: &str) {
        for c in s.chars() {
            self.insert(c);
        }
    }
    fn backspace(&mut self) -> Option<char> {
        if self.gap_start == 0 {
            return None;
        }
        self.gap_start -= 1;
        Some(self.buf[self.gap_start])
    }
    fn delete(&mut self) -> Option<char> {
        if self.gap_end == self.buf.len() {
            return None;
        }
        self.gap_end += 1;
        Some(self.buf[self.gap_end - 1])
    }
    fn char_at(&self, i: usize) -> Option<char> {
        if i < self.gap_start {
            Some(self.buf[i])
        } else if i < self.len() {
            Some(self.buf[i + self.gap_len()])
        } else {
            None
        }
    }
    fn line_col(&self, pos: usize) -> (usize, usize) {
        let mut line = 0;
        let mut col = 0;
        for i in 0..pos.min(self.len()) {
            if self.char_at(i) == Some('\n') {
                line += 1;
                col = 0;
            } else {
                col += 1;
            }
        }
        (line, col)
    }
}

impl fmt::Display for GapBuffer {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let s: String = self.buf[..self.gap_start].iter().chain(&self.buf[self.gap_end..]).collect();
        f.write_str(&s)
    }
}

#[derive(Debug, Clone)]
enum Edit {
    Insert { at: usize, text: String },
    Delete { at: usize, text: String },
}

struct Editor {
    gb: GapBuffer,
    undo: Vec<Edit>,
}

impl Editor {
    fn type_at(&mut self, at: usize, text: &str) {
        self.gb.move_to(at);
        self.gb.insert_str(text);
        self.undo.push(Edit::Insert { at, text: text.to_string() });
    }
    fn delete_range(&mut self, at: usize, n: usize) {
        self.gb.move_to(at);
        let mut removed = String::new();
        for _ in 0..n {
            if let Some(c) = self.gb.delete() {
                removed.push(c);
            }
        }
        self.undo.push(Edit::Delete { at, text: removed });
    }
    fn undo(&mut self) -> Option<Edit> {
        let e = self.undo.pop()?;
        match &e {
            Edit::Insert { at, text } => {
                self.gb.move_to(at + text.chars().count());
                for _ in 0..text.chars().count() {
                    self.gb.backspace();
                }
            }
            Edit::Delete { at, text } => {
                self.gb.move_to(*at);
                self.gb.insert_str(text);
            }
        }
        Some(e)
    }
}

fn main() {
    let mut gb = GapBuffer::new(4);
    gb.insert_str("Hello world");
    println!("{:?} len={} cursor={} cap={}", gb.to_string(), gb.len(), gb.cursor(), gb.buf.len());
    gb.move_to(5);
    gb.insert(',');
    println!("{:?} cursor={}", gb.to_string(), gb.cursor());
    gb.move_to(0);
    gb.insert_str(">> ");
    gb.move_to(100);
    gb.insert('!');
    println!("{:?}", gb.to_string());
    gb.move_to(3);
    println!("del {:?} {:?} back {:?}", gb.delete(), gb.delete(), gb.backspace());
    println!("{:?} cursor={}", gb.to_string(), gb.cursor());
    println!("chars: {:?} {:?} {:?}", gb.char_at(0), gb.char_at(5), gb.char_at(99));

    let mut ed = Editor { gb: GapBuffer::new(16), undo: vec![] };
    ed.type_at(0, "fn main() {\n}\n");
    ed.type_at(12, "    println!(\"hi\");\n");
    println!("---\n{}---", ed.gb);
    let pos = ed.gb.to_string().find("println").unwrap();
    println!("println at {} -> line/col {:?}", pos, ed.gb.line_col(pos));
    ed.delete_range(pos, 8);
    ed.type_at(pos, "eprintln!");
    println!("---\n{}---", ed.gb);
    while let Some(e) = ed.undo() {
        println!("undid {:?}", e);
        println!("  now {:?}", ed.gb.to_string());
    }
    let text = "héllo wörld";
    let mut g = GapBuffer::new(2);
    g.insert_str(text);
    g.move_to(6);
    g.insert_str("NEW ");
    println!("{} len={}", g, g.len());
}
