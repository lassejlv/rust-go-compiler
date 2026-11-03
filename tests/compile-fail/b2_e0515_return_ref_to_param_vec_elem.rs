//~ ERROR E0515
fn first_owned(v: Vec<String>) -> &'static String {
    &v[0]
}
fn main() { println!("{}", first_owned(vec!["a".into()])); }
