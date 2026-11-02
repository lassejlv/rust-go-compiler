const TABLE: &[(char, &[char])] = &[('a', &['A']), ('k', &['K', '\u{212A}']), ('s', &['S', 'ſ'])];
static PAIRS: &[(&str, &[u8])] = &[("x", &[1, 2]), ("y", &[])];
fn find(c: char) -> &'static [char] { TABLE.iter().find(|(k, _)| *k == c).map(|(_, v)| *v).unwrap_or(&[]) }
fn main() { println!("{:?} {:?} {:?} {:?}", find('k'), find('s'), find('z'), PAIRS); }
