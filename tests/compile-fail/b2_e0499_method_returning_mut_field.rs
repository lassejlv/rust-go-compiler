//~ ERROR E0499
struct Bank { accounts: Vec<i64> }
impl Bank {
    fn acct(&mut self, i: usize) -> &mut i64 { &mut self.accounts[i] }
}
fn transfer(b: &mut Bank) {
    let from = b.acct(0);
    let to = b.acct(1);
    *from -= 10;
    *to += 10;
}
fn main() { let mut b = Bank { accounts: vec![10, 0] }; transfer(&mut b); }
