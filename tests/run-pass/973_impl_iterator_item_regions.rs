// The items of an `impl Iterator<Item = (&'s T, &'a U)>` result have the
// regions the binding declares: the `&'s T` half does not hold the borrow
// of `self` ('a), also when the hidden type stores the items at the same
// reference depth (`vec::IntoIter<(&'s T, &'a U)>`).

struct Store<'s, T> {
    names: Vec<&'s T>,
    scores: Vec<u32>,
}

impl<'s, T: std::fmt::Debug> Store<'s, T> {
    fn pairs<'a>(&'a self) -> impl Iterator<Item = (&'s T, &'a u32)> + 'a {
        let v: Vec<(&'s T, &'a u32)> = self.names.iter().copied().zip(self.scores.iter()).collect();
        v.into_iter()
    }
    fn zipped<'a>(&'a self) -> impl Iterator<Item = (&'s T, &'a u32)> + 'a {
        self.names.iter().copied().zip(self.scores.iter())
    }
    fn best(&self) -> Option<(&'s T, &u32)> {
        self.pairs().max_by_key(|p| *p.1)
    }
    fn best_name(&self) -> Option<&'s T> {
        self.pairs().max_by_key(|p| *p.1).map(|p| p.0)
    }
    fn first(&self) -> &'s T {
        let (n, _) = self.pairs().next().unwrap();
        n
    }
    fn last_zipped(&self) -> Option<&'s T> {
        self.zipped().last().map(|(n, _)| n)
    }
}

fn names<'s>(st: &Store<'s, String>) -> Vec<&'s String> {
    st.pairs().map(|(n, _)| n).collect()
}

fn singles<'s, 'v>(v: &'v [&'s str]) -> impl Iterator<Item = &'s str> + 'v {
    let w: Vec<&'s str> = v.iter().copied().filter(|s| s.len() == 1).collect();
    w.into_iter()
}

fn main() {
    let a = String::from("ann");
    let b = String::from("bob");
    let best;
    let all;
    let f;
    let l;
    {
        let st = Store { names: vec![&a, &b], scores: vec![3, 9] };
        println!("{:?}", st.best());
        best = st.best_name();
        f = st.first();
        l = st.last_zipped();
        all = names(&st);
    }
    println!("{:?} {:?} {} {:?}", best, all, f, l);

    let text = String::from("a bb c ddd");
    let one;
    {
        let words: Vec<&str> = text.split(' ').collect();
        one = singles(&words).last();
    }
    println!("{:?}", one);
}
