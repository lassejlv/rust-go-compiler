//~ ERROR E0392
enum Token<'a> {
    Word(String),
    Number(i64),
}
fn main() {}
