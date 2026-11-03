// Edition 2021 precise captures: closures capture disjoint fields.
struct State {
    counter: u32,
    log: Vec<String>,
    limit: u32,
}

fn main() {
    let mut st = State { counter: 0, log: Vec::new(), limit: 3 };
    let mut inc = || st.counter += 1;
    st.log.push(String::from("start"));
    inc();
    inc();
    let limit = st.limit;
    let mut record = |m: &str| st.log.push(format!("{} (limit {})", m, limit));
    st.counter += 10;
    record("a");
    record("b");
    let read_limit = || st.limit;
    st.counter += read_limit();
    println!("{} {:?}", st.counter, st.log);
}
