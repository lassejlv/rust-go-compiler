use std::collections::HashMap;
struct Interner<'s> { map: HashMap<&'s str, u32>, names: Vec<&'s str> }
impl<'s> Interner<'s> {
    fn intern(&mut self, s: &'s str) -> u32 {
        if let Some(&id) = self.map.get(s) { return id; }
        let id = self.names.len() as u32;
        self.map.insert(s, id);
        self.names.push(s);
        id
    }
    fn name(&self, id: u32) -> &'s str { self.names[id as usize] }
}
struct Parser<'s, 'i> { toks: std::str::SplitWhitespace<'s>, interner: &'i mut Interner<'s>, out: Vec<u32> }
impl<'s, 'i> Parser<'s, 'i> {
    fn run(mut self) -> Vec<u32> {
        while let Some(t) = self.toks.next() { let id = self.interner.intern(t); self.out.push(id); }
        self.out
    }
    fn peek_name(&self) -> Option<&'s str> { self.out.last().map(|&id| self.interner.name(id)) }
}
struct Cursor<'a, T> { items: &'a [T], pos: usize }
impl<'a, T> Cursor<'a, T> {
    fn next_item(&mut self) -> Option<&'a T> { let it = self.items.get(self.pos)?; self.pos += 1; Some(it) }
    fn rest(&self) -> &'a [T] { &self.items[self.pos..] }
    fn take_while<F: FnMut(&T) -> bool>(&mut self, mut f: F) -> &'a [T] { let st = self.pos; while self.pos < self.items.len() && f(&self.items[self.pos]) { self.pos += 1; } &self.items[st..self.pos] }
}
struct Holder<'a> { parts: Vec<&'a str>, sep: &'a str }
impl<'a> Holder<'a> {
    fn add(&mut self, p: &'a str) -> &mut Holder<'a> { self.parts.push(p); self }
    fn joined(&self) -> String { self.parts.join(self.sep) }
    fn longest(&self) -> &'a str { self.parts.iter().copied().max_by_key(|p| p.len()).unwrap_or("") }
}
fn main() {
    let text = String::from("a b a c b d a");
    let mut int = Interner { map: HashMap::new(), names: vec![] };
    let p = Parser { toks: text.split_whitespace(), interner: &mut int, out: vec![] };
    println!("{:?}", p.peek_name());
    let ids = p.run();
    println!("{:?} {:?}", ids, ids.iter().map(|&i| int.name(i)).collect::<String>());
    let data = [1, 2, 3, 10, 11, 4];
    let mut c = Cursor { items: &data, pos: 0 };
    let a = c.next_item();
    let small = c.take_while(|&x| x < 10);
    let rest = c.rest();
    println!("{:?} {:?} {:?}", a, small, rest);
    let s1 = String::from("alpha");
    let mut h = Holder { parts: vec![], sep: "-" };
    h.add(&s1).add("be").add(&text[2..5]);
    let l = h.longest();
    println!("{} {}", h.joined(), l);
}
