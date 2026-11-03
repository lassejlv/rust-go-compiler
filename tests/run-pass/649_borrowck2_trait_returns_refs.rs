// Valid programs that a naive borrow checker / front end might reject.
// Each module is an independent program; main runs them in order.

mod p11_trait_returns_refs {
    trait Named { fn name(&self) -> &str; fn tags(&self) -> &[String]; }
    struct User { n: String, t: Vec<String> }
    impl Named for User {
        fn name(&self) -> &str { &self.n }
        fn tags(&self) -> &[String] { &self.t }
    }
    trait Store { type Item; fn get(&self, i: usize) -> Option<&Self::Item>; fn get_mut(&mut self, i: usize) -> Option<&mut Self::Item>; }
    struct VS(Vec<u32>);
    impl Store for VS {
        type Item = u32;
        fn get(&self, i: usize) -> Option<&u32> { self.0.get(i) }
        fn get_mut(&mut self, i: usize) -> Option<&mut u32> { self.0.get_mut(i) }
    }
    fn longest_name<'a>(xs: &'a [Box<dyn Named>]) -> &'a str {
        xs.iter().map(|x| x.name()).max_by_key(|n| n.len()).unwrap_or("")
    }
    fn double_first<S: Store<Item = u32>>(s: &mut S) -> u32 {
        if let Some(x) = s.get_mut(0) { *x *= 2; }
        *s.get(0).unwrap()
    }
    pub fn main() {
        let us: Vec<Box<dyn Named>> = vec![Box::new(User { n: "ann".into(), t: vec!["x".into()] }), Box::new(User { n: "robert".into(), t: vec![] })];
        let n = longest_name(&us);
        let t: Vec<&String> = us.iter().flat_map(|u| u.tags()).collect();
        let mut vs = VS(vec![21, 1]);
        println!("{} {:?} {}", n, t, double_first(&mut vs));
    }
}

mod p58_generic_lifetime_traits_impl_for_ref {
    use std::fmt::Display;
    trait Describe { fn describe(&self) -> String; }
    impl<'a, T: Display> Describe for &'a T { fn describe(&self) -> String { format!("ref({})", self) } }
    impl Describe for i32 { fn describe(&self) -> String { format!("i32({})", self) } }
    trait Source<'a> { fn read(&self) -> &'a str; }
    struct Fixed<'a>(&'a str);
    impl<'a> Source<'a> for Fixed<'a> { fn read(&self) -> &'a str { self.0 } }
    fn read_all<'a, S: Source<'a>>(srcs: &[S]) -> Vec<&'a str> { srcs.iter().map(|s| s.read()).collect() }
    pub fn main() {
        let x = 5i32;
        let rx = &x;
        let owned = String::from("dyn");
        let out;
        {
            let srcs = vec![Fixed(owned.as_str()), Fixed("lit")];
            out = read_all(&srcs);
        }
        println!("{} {} {:?}", x.describe(), (&rx).describe(), out);
    }
}

mod p50_traits_assoc_types_generic {
    use std::fmt::Debug;
    trait Container { type Item; fn items(&self) -> &[Self::Item]; fn first(&self) -> Option<&Self::Item> { self.items().first() } }
    struct Bag<T> { v: Vec<T> }
    impl<T> Container for Bag<T> { type Item = T; fn items(&self) -> &[T] { &self.v } }
    fn debug_first<C: Container>(c: &C) -> String where C::Item: Debug { format!("{:?}", c.first()) }
    fn largest<C>(c: &C) -> Option<&C::Item> where C: Container, C::Item: Ord { c.items().iter().max() }
    trait Animal { fn name(&self) -> String; }
    trait Pet: Animal { fn greet(&self) -> String { format!("hi {}", self.name()) } }
    struct Dog;
    impl Animal for Dog { fn name(&self) -> String { "dog".into() } }
    impl Pet for Dog {}
    fn as_animal(p: &dyn Pet) -> &dyn Animal { p }
    impl<T: Debug> Bag<T> { fn dump(&self) -> String { format!("{:?}", self.v) } }
    pub fn main() {
        let b = Bag { v: vec![3, 9, 2] };
        let d = Dog;
        println!("{} {:?} {} {} {}", debug_first(&b), largest(&b), b.dump(), d.greet(), as_animal(&d).name());
    }
}

fn main() {
    p11_trait_returns_refs::main();
    p58_generic_lifetime_traits_impl_for_ref::main();
    p50_traits_assoc_types_generic::main();
}
