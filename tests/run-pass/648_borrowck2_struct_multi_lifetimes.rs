// Valid programs that a naive borrow checker / front end might reject.
// Each module is an independent program; main runs them in order.

mod p10_struct_multi_lifetimes {
    struct Pair<'a, 'b> { a: &'a str, b: &'b str }
    impl<'a, 'b> Pair<'a, 'b> {
        fn get_a(&self) -> &'a str { self.a }
        fn with_b<'c>(self, b: &'c str) -> Pair<'a, 'c> { Pair { a: self.a, b } }
    }
    struct Parser<'src> { src: &'src str, pos: usize }
    impl<'src> Parser<'src> {
        fn next_word(&mut self) -> Option<&'src str> {
            let rest = &self.src[self.pos..];
            let rest_trim = rest.trim_start();
            if rest_trim.is_empty() { return None; }
            let start = self.pos + (rest.len() - rest_trim.len());
            let end = rest_trim.find(' ').map(|i| start + i).unwrap_or(self.src.len());
            self.pos = end;
            Some(&self.src[start..end])
        }
    }
    pub fn main() {
        let long = String::from("long-lived");
        let a_ref;
        {
            let short = String::from("short");
            let p = Pair { a: &long, b: &short };
            a_ref = p.get_a();
            let p2 = p.with_b("static");
            println!("{} {}", p2.a, p2.b);
        }
        let text = String::from("  the quick  fox ");
        let mut words = Vec::new();
        {
            let mut p = Parser { src: &text, pos: 0 };
            while let Some(w) = p.next_word() { words.push(w); }
        }
        println!("{} {:?}", a_ref, words);
    }
}

mod p51_two_lifetime_params_methods {
    struct Ctx<'s, 'c> { src: &'s str, cfg: &'c Vec<String> }
    impl<'s, 'c> Ctx<'s, 'c> {
        fn word(&self, i: usize) -> &'s str { self.src.split(' ').nth(i).unwrap_or("") }
        fn opt(&self, i: usize) -> &'c str { &self.cfg[i] }
    }
    pub fn main() {
        let src = String::from("alpha beta");
        let w;
        {
            let cfg = vec![String::from("o")];
            let c = Ctx { src: &src, cfg: &cfg };
            w = c.word(1);
            println!("{}", c.opt(0));
        }
        println!("{}", w);
    }
}

mod p61_tuple_pairs_independent_loans {
    use std::collections::HashMap;
    fn first_of<'a, 'b>(p: (&'a str, &'b str)) -> &'a str { p.0 }
    pub fn main() {
        let mut a = String::from("a");
        let b = String::from("b");
        let (x, _y) = (&b, &a);
        a.push('!');
        let t = (&b, &a);
        let z = t.0;
        a.push('?');
        let v = vec![(&b, &a)];
        let w = v[0].0;
        a.push('#');
        let k = String::from("k");
        let mut val = String::from("v");
        let m: HashMap<&str, &str> = HashMap::from([(k.as_str(), val.as_str())]);
        let key: &str = *m.keys().next().unwrap();
        val.push('!');
        let f = first_of((&b, &a));
        a.push('$');
        println!("{} {} {} {} {} {} {}", x, z, w, key, f, a, val);
    }
}

fn main() {
    p10_struct_multi_lifetimes::main();
    p51_two_lifetime_params_methods::main();
    p61_tuple_pairs_independent_loans::main();
}
