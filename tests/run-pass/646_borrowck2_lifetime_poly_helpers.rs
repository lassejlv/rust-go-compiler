// Valid programs that a naive borrow checker / front end might reject.
// Each module is an independent program; main runs them in order.

mod p08_lifetime_poly_helpers {
    fn first<'a, T>(v: &'a [T]) -> Option<&'a T> { v.first() }
    fn longer<'a>(a: &'a str, b: &'a str) -> &'a str { if a.len() >= b.len() { a } else { b } }
    fn split_pair<'a>(s: &'a str, sep: char) -> (&'a str, &'a str) {
        match s.find(sep) { Some(i) => (&s[..i], &s[i + 1..]), None => (s, "") }
    }
    fn pick_mut<'a, T>(a: &'a mut T, b: &'a mut T, first: bool) -> &'a mut T { if first { a } else { b } }
    pub fn main() {
        let words = vec![String::from("alpha"), String::from("be")];
        let mut best: &str = "";
        for w in &words { best = longer(best, w); }
        let outer = String::from("k=v");
        let (k, v);
        {
            let parts = split_pair(&outer, '=');
            k = parts.0; v = parts.1;
        }
        let (mut a, mut b) = (1, 2);
        *pick_mut(&mut a, &mut b, false) += 10;
        *pick_mut(&mut a, &mut b, true) += 5;
        println!("{:?} {} {} {} {} {}", first(&words), best, k, v, a, b);
    }
}

mod p42_implied_bounds_nested_refs {
    fn deref_outer<'a, 'b>(x: &'a &'b str) -> &'b str { *x }
    fn pick_inner<'a, 'b, T>(x: &'a mut &'b T) -> &'b T { *x }
    struct Ref<'a, T>(&'a T);
    fn unwrap_ref<'a, T>(r: Ref<'a, T>) -> &'a T { r.0 }
    fn nested<'a>(v: &'a Vec<&'a str>) -> Vec<&'a str> { v.iter().map(|s| *s).collect() }
    pub fn main() {
        let s = String::from("inner");
        let outer_ref: &str;
        {
            let r = &s[..];
            let rr = &r;
            outer_ref = deref_outer(rr);
        }
        let n = 3;
        let mut nr = &n;
        let got = pick_inner(&mut nr);
        let w = Ref(&n);
        let words = vec!["a", "b"];
        println!("{} {} {} {:?}", outer_ref, got, unwrap_ref(w), nested(&words));
    }
}

mod p32_variance_and_bounds {
    use std::borrow::Cow;
    use std::fmt::Debug;
    fn shortest<'a>(v: &[&'a str]) -> &'a str { v.iter().copied().min_by_key(|s| s.len()).unwrap() }
    fn show_all<'a, T: Debug + 'a>(items: impl IntoIterator<Item = &'a T>) -> String {
        items.into_iter().map(|t| format!("{:?}", t)).collect::<Vec<_>>().join("|")
    }
    fn normalize(s: &str) -> Cow<'_, str> {
        if s.contains(' ') { Cow::Owned(s.replace(' ', "_")) } else { Cow::Borrowed(s) }
    }
    struct Wrap<'a, T: 'a> { r: &'a T }
    impl<'a, T: Debug> Wrap<'a, T> { fn get(&self) -> &'a T { self.r } }
    fn outlives<'a: 'b, 'b>(x: &'a i32, _y: &'b i32) -> &'b i32 { x }
    pub fn main() {
        let statics: Vec<&'static str> = vec!["hello", "hi", "hey"];
        let owned = String::from("h");
        let mut mixed: Vec<&str> = statics.clone();
        mixed.push(&owned);
        let s = shortest(&mixed);
        let n = 5;
        let w = Wrap { r: &n };
        let got = w.get();
        let m = 6;
        let o = outlives(got, &m);
        let c1 = normalize("a b c");
        let c2 = normalize("abc");
        println!("{} {} {} {} {}", s, show_all(&[1, 2, 3]), o, c1, c2);
        println!("{}", matches!(c1, Cow::Owned(_)) && matches!(c2, Cow::Borrowed(_)));
    }
}

fn main() {
    p08_lifetime_poly_helpers::main();
    p42_implied_bounds_nested_refs::main();
    p32_variance_and_bounds::main();
}
