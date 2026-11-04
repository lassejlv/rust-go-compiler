mod h02 {
    struct Library { books: Vec<Book> }
    #[derive(Debug, Clone)] struct Book { title: String, year: u16, tags: Vec<&'static str> }
    impl Library {
        fn by_tag<'a>(&'a self, tag: &'a str) -> impl Iterator<Item = &'a Book> + 'a { self.books.iter().filter(move |b| b.tags.contains(&tag)) }
        fn titles(&self) -> impl Iterator<Item = &str> { self.books.iter().map(|b| b.title.as_str()) }
        fn oldest(&self) -> Option<&Book> { self.books.iter().min_by_key(|b| b.year) }
        fn longest_title(&self) -> Option<&str> { self.titles().max_by_key(|t| t.len()) }
        fn decades(&self) -> std::collections::BTreeMap<u16, Vec<&str>> { let mut m = std::collections::BTreeMap::new(); for b in &self.books { m.entry(b.year / 10 * 10).or_insert_with(Vec::new).push(&b.title[..]); } m }
        fn year_mut(&mut self, t: &str) -> Option<&mut u16> { self.books.iter_mut().find(|b| b.title == t).map(|b| &mut b.year) }
    }
    pub fn main() {
        let mut lib = Library { books: vec![
            Book { title: "Dune".into(), year: 1965, tags: vec!["scifi", "classic"] },
            Book { title: "Neuromancer".into(), year: 1984, tags: vec!["scifi", "cyber"] },
            Book { title: "Emma".into(), year: 1815, tags: vec!["classic"] },
        ] };
        println!("{:?}", lib.by_tag("classic").map(|b| &b.title).collect::<Vec<_>>());
        println!("{:?} {:?}", lib.oldest().map(|b| b.year), lib.longest_title());
        println!("{:?}", lib.decades());
        if let Some(y) = lib.year_mut("Emma") { *y += 1; }
        let mut sorted: Vec<&Book> = lib.books.iter().collect();
        sorted.sort_by_key(|b| std::cmp::Reverse(b.year));
        println!("{:?}", sorted.iter().map(|b| (b.title.as_str(), b.year)).collect::<Vec<_>>());
        let mut titles: Vec<&str> = lib.titles().collect();
        titles.sort_by_key(|t| t.to_lowercase());
        println!("{:?}", titles);
        let owned: Vec<Book> = lib.by_tag("scifi").cloned().collect();
        drop(lib);
        println!("{}", owned.len());
    }
}

mod h03 {
    use std::rc::Rc;
    use std::cell::RefCell;
    type Callback<'a> = Box<dyn FnMut(&str) -> bool + 'a>;
    struct Bus<'a> { subs: Vec<(String, Callback<'a>)> }
    impl<'a> Bus<'a> {
        fn on(&mut self, topic: &str, cb: impl FnMut(&str) -> bool + 'a) { self.subs.push((topic.to_string(), Box::new(cb))); }
        fn emit(&mut self, topic: &str, msg: &str) -> usize { let mut n = 0; self.subs.retain_mut(|(t, cb)| { if t == topic { n += 1; cb(msg) } else { true } }); n }
    }
    pub fn main() {
        let log = Rc::new(RefCell::new(Vec::<String>::new()));
        let mut count = 0;
        {
            let mut bus = Bus { subs: vec![] };
            let l1 = log.clone();
            bus.on("a", move |m| { l1.borrow_mut().push(format!("a1:{}", m)); true });
            let l2 = Rc::clone(&log);
            bus.on("a", move |m| { l2.borrow_mut().push(format!("a2:{}", m)); m != "stop" });
            bus.on("b", |m| { count += m.len(); true });
            println!("{}", bus.emit("a", "hi"));
            println!("{}", bus.emit("a", "stop"));
            println!("{}", bus.emit("a", "again"));
            println!("{}", bus.emit("b", "four"));
            println!("{}", bus.emit("c", "x"));
        }
        println!("{:?} {}", log.borrow(), count);
        println!("{}", Rc::strong_count(&log));
    }
}

fn main() {
    println!("== h02");
    h02::main();
    println!("== h03");
    h03::main();
}
