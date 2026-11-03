use std::collections::HashMap;
struct Arena<'s> { words: Vec<&'s str>, by_len: HashMap<usize, Vec<&'s str>> }
impl<'s> Arena<'s> {
    fn new(text: &'s str) -> Self {
        let words: Vec<&'s str> = text.split_whitespace().collect();
        let mut by_len: HashMap<usize, Vec<&'s str>> = HashMap::new();
        for w in &words { by_len.entry(w.len()).or_default().push(w); }
        Arena { words, by_len }
    }
    fn longest(&self) -> Option<&'s str> { self.words.iter().copied().max_by_key(|w| w.len()) }
    fn with_len(&self, n: usize) -> &[&'s str] { self.by_len.get(&n).map(|v| &v[..]).unwrap_or(&[]) }
    fn pairs<'a>(&'a self) -> impl Iterator<Item = (&'s str, &'s str)> + 'a { self.words.windows(2).map(|w| (w[0], w[1])) }
    fn find_by<F>(&self, f: F) -> Vec<(usize, &'s str)> where F: Fn(&str) -> bool { self.words.iter().enumerate().filter(|(_, w)| f(w)).map(|(i, w)| (i, *w)).collect() }
    fn split_at_word(&self, w: &str) -> (&[&'s str], &[&'s str]) { let i = self.words.iter().position(|x| *x == w).unwrap_or(self.words.len()); self.words.split_at(i) }
}
fn pick<'a, 'b>(a: &'a str, b: &'b str, first: bool) -> &'a str where 'b: 'a { if first { a } else { b } }
struct Ctx<'a, 'b> { name: &'a str, data: &'b mut Vec<&'a str> }
impl<'a, 'b> Ctx<'a, 'b> {
    fn push(&mut self, s: &'a str) -> &mut Self { self.data.push(s); self }
    fn last(&self) -> Option<&'a str> { self.data.last().copied() }
    fn name_and_last(&self) -> (&'a str, Option<&&'a str>) { (self.name, self.data.last()) }
}
fn main() {
    let text = String::from("the quick brown fox jumps over the lazy dog quickly");
    let a = Arena::new(&text);
    println!("{:?} {:?} {:?}", a.longest(), a.with_len(5), a.with_len(9));
    println!("{:?}", a.pairs().filter(|(x, y)| x.len() == y.len()).collect::<Vec<_>>());
    println!("{:?}", a.find_by(|w| w.contains('o')));
    let (l, r) = a.split_at_word("over");
    println!("{:?} | {:?}", l, r);
    let owned = String::from("owned");
    println!("{} {}", pick(&text, &owned, true), pick(&text[..3], "static", false));
    let mut store = vec![];
    let mut ctx = Ctx { name: "ctx", data: &mut store };
    ctx.push("a").push(&text[4..9]);
    let last = ctx.last();
    println!("{:?} {:?}", last, ctx.name_and_last());
    drop(ctx);
    store.push("z");
    println!("{:?} {:?}", store, last);
    let longest_of_all = { let a2 = Arena::new(&text); a2.longest().unwrap() };
    println!("{}", longest_of_all);
    let mut cache: HashMap<&str, usize> = HashMap::new();
    for w in a.words.iter() { *cache.entry(w).or_insert(0) += 1; }
    let mut top: Vec<(&str, usize)> = cache.into_iter().collect();
    top.sort_by(|x, y| y.1.cmp(&x.1).then(x.0.cmp(y.0)));
    println!("{:?}", &top[..3]);
}
