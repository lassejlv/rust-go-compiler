// Double-entry bookkeeping with a fixed-point Decimal type (i128 mantissa,
// scale 4), banker's rounding, currency conversion, parsing with errors,
// thousands separators in Display, journal validation, trial balance,
// FIFO cost-basis lots for an investment account, and interest accrual.
use std::collections::{BTreeMap, HashMap, VecDeque};
use std::fmt;
use std::iter::Sum;
use std::ops::{Add, AddAssign, Div, Mul, Neg, Sub, SubAssign};
use std::str::FromStr;

const SCALE: i128 = 10_000;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
struct Dec(i128);

#[derive(Debug, Clone, Copy, PartialEq)]
enum Rounding {
    HalfEven,
    HalfUp,
    Down,
}

impl Dec {
    const ZERO: Dec = Dec(0);
    fn from_int(i: i64) -> Dec {
        Dec(i as i128 * SCALE)
    }
    fn round_to(self, places: u32, mode: Rounding) -> Dec {
        let unit = 10i128.pow(4 - places);
        let q = self.0.div_euclid(unit);
        let r = self.0.rem_euclid(unit);
        let half = unit / 2;
        let up = match mode {
            Rounding::Down => {
                if self.0 < 0 && r != 0 {
                    1
                } else {
                    0
                }
            }
            Rounding::HalfUp => {
                if self.0 >= 0 {
                    (r >= half) as i128
                } else {
                    (r > half) as i128
                }
            }
            Rounding::HalfEven => {
                if r > half || (r == half && q % 2 != 0) {
                    1
                } else {
                    0
                }
            }
        };
        Dec((q + up) * unit)
    }
    fn abs(self) -> Dec {
        Dec(self.0.abs())
    }
    fn mul_rounded(self, o: Dec) -> Dec {
        // exact product has scale 8; round half-even back to scale 4
        let p = self.0 * o.0;
        let q = p.div_euclid(SCALE);
        let r = p.rem_euclid(SCALE);
        let up = r > SCALE / 2 || (r == SCALE / 2 && q % 2 != 0);
        Dec(q + up as i128)
    }
}

impl fmt::Display for Dec {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let places = f.precision().unwrap_or(2).min(4) as u32;
        let v = self.round_to(places, Rounding::HalfEven).0;
        let neg = v < 0;
        let v = v.abs();
        let int = v / SCALE;
        let frac = (v % SCALE) / 10i128.pow(4 - places);
        let digits = int.to_string();
        let mut grouped = String::new();
        for (i, ch) in digits.chars().enumerate() {
            if i > 0 && (digits.len() - i) % 3 == 0 && f.alternate() {
                grouped.push(',');
            }
            grouped.push(ch);
        }
        let body = if places > 0 { format!("{}.{:0width$}", grouped, frac, width = places as usize) } else { grouped };
        f.pad_integral(!neg, "", &body)
    }
}

impl fmt::Debug for Dec {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Dec({:.4})", self)
    }
}

#[derive(Debug, PartialEq)]
enum DecError {
    Empty,
    BadChar(char),
    TooPrecise(usize),
    Overflow,
}

impl FromStr for Dec {
    type Err = DecError;
    fn from_str(s: &str) -> Result<Dec, DecError> {
        let s = s.trim();
        let (neg, s) = match s.strip_prefix('-') {
            Some(r) => (true, r),
            None => (false, s.strip_prefix('+').unwrap_or(s)),
        };
        if s.is_empty() {
            return Err(DecError::Empty);
        }
        let (ip, fp) = s.split_once('.').unwrap_or((s, ""));
        if fp.len() > 4 {
            return Err(DecError::TooPrecise(fp.len()));
        }
        let mut v: i128 = 0;
        for c in ip.chars().filter(|&c| c != '_' && c != ',') {
            let d = c.to_digit(10).ok_or(DecError::BadChar(c))?;
            v = v.checked_mul(10).and_then(|v| v.checked_add(d as i128)).ok_or(DecError::Overflow)?;
        }
        let mut frac: i128 = 0;
        for (i, c) in fp.chars().enumerate() {
            let d = c.to_digit(10).ok_or(DecError::BadChar(c))? as i128;
            frac += d * 10i128.pow(3 - i as u32);
        }
        let total = v.checked_mul(SCALE).ok_or(DecError::Overflow)? + frac;
        Ok(Dec(if neg { -total } else { total }))
    }
}

impl Add for Dec {
    type Output = Dec;
    fn add(self, o: Dec) -> Dec {
        Dec(self.0 + o.0)
    }
}
impl Sub for Dec {
    type Output = Dec;
    fn sub(self, o: Dec) -> Dec {
        Dec(self.0 - o.0)
    }
}
impl Neg for Dec {
    type Output = Dec;
    fn neg(self) -> Dec {
        Dec(-self.0)
    }
}
impl AddAssign for Dec {
    fn add_assign(&mut self, o: Dec) {
        self.0 += o.0;
    }
}
impl SubAssign for Dec {
    fn sub_assign(&mut self, o: Dec) {
        self.0 -= o.0;
    }
}
impl Mul<i64> for Dec {
    type Output = Dec;
    fn mul(self, k: i64) -> Dec {
        Dec(self.0 * k as i128)
    }
}
impl Div<i64> for Dec {
    type Output = Dec;
    fn div(self, k: i64) -> Dec {
        // half-even division
        let k = k as i128;
        let q = self.0.div_euclid(k);
        let r = self.0.rem_euclid(k);
        let up = 2 * r > k || (2 * r == k && q % 2 != 0);
        Dec(q + up as i128)
    }
}
impl Sum for Dec {
    fn sum<I: Iterator<Item = Dec>>(it: I) -> Dec {
        it.fold(Dec::ZERO, |a, b| a + b)
    }
}
impl<'a> Sum<&'a Dec> for Dec {
    fn sum<I: Iterator<Item = &'a Dec>>(it: I) -> Dec {
        it.copied().sum()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
enum Kind {
    Asset,
    Liability,
    Equity,
    Income,
    Expense,
}

impl Kind {
    fn normal_debit(self) -> bool {
        matches!(self, Kind::Asset | Kind::Expense)
    }
}

#[derive(Debug, Clone)]
struct Posting {
    account: &'static str,
    amount: Dec, // positive = debit, negative = credit
    currency: &'static str,
}

#[derive(Debug)]
struct Txn {
    date: (u16, u8, u8),
    memo: String,
    postings: Vec<Posting>,
}

#[derive(Debug, PartialEq)]
enum LedgerError {
    Unbalanced { memo: String, currency: &'static str, off_by: Dec },
    UnknownAccount(&'static str),
    Empty(String),
}

struct Ledger {
    accounts: BTreeMap<&'static str, Kind>,
    balances: HashMap<(&'static str, &'static str), Dec>,
    journal: Vec<Txn>,
    rates: HashMap<(&'static str, &'static str), Dec>,
}

impl Ledger {
    fn post(&mut self, t: Txn) -> Result<(), LedgerError> {
        if t.postings.is_empty() {
            return Err(LedgerError::Empty(t.memo));
        }
        let mut per_ccy: BTreeMap<&'static str, Dec> = BTreeMap::new();
        for p in &t.postings {
            if !self.accounts.contains_key(p.account) {
                return Err(LedgerError::UnknownAccount(p.account));
            }
            *per_ccy.entry(p.currency).or_default() += p.amount;
        }
        if let Some((ccy, off)) = per_ccy.iter().find(|(_, v)| **v != Dec::ZERO) {
            return Err(LedgerError::Unbalanced { memo: t.memo.clone(), currency: ccy, off_by: *off });
        }
        for p in &t.postings {
            *self.balances.entry((p.account, p.currency)).or_default() += p.amount;
        }
        self.journal.push(t);
        Ok(())
    }

    fn convert(&self, amt: Dec, from: &'static str, to: &'static str) -> Option<Dec> {
        if from == to {
            return Some(amt);
        }
        if let Some(r) = self.rates.get(&(from, to)) {
            return Some(amt.mul_rounded(*r));
        }
        // inverse rate: divide with half-even rounding
        let inv = self.rates.get(&(to, from))?;
        Some(Dec((amt.0 * SCALE * 2 + inv.0) / (inv.0 * 2)))
    }

    fn trial_balance(&self, report_ccy: &'static str) -> (Vec<String>, Dec, Dec) {
        let mut lines = vec![];
        let (mut dr, mut cr) = (Dec::ZERO, Dec::ZERO);
        let mut keys: Vec<_> = self.balances.keys().copied().collect();
        keys.sort();
        for (acct, ccy) in keys {
            let bal = self.balances[&(acct, ccy)];
            if bal == Dec::ZERO {
                continue;
            }
            let conv = self.convert(bal, ccy, report_ccy).expect("rate");
            let kind = self.accounts[acct];
            if conv.0 >= 0 {
                dr += conv;
            } else {
                cr += -conv;
            }
            let sign_ok = (conv.0 >= 0) == kind.normal_debit();
            lines.push(format!("{:<22} {:>3} {:>14.2} {:>14.2} {:?}{}", acct, ccy, bal, conv, kind, if sign_ok { "" } else { " (contra)" }));
        }
        (lines, dr, cr)
    }
}

struct Lot {
    qty: i64,
    cost: Dec,
}

fn fifo_sell(lots: &mut VecDeque<Lot>, mut qty: i64, price: Dec) -> Result<Dec, String> {
    let have: i64 = lots.iter().map(|l| l.qty).sum();
    if qty > have {
        return Err(format!("cannot sell {} shares, only {} held", qty, have));
    }
    let mut gain = Dec::ZERO;
    while qty > 0 {
        let front = lots.front_mut().unwrap();
        let take = qty.min(front.qty);
        gain += (price - front.cost) * take;
        front.qty -= take;
        qty -= take;
        if front.qty == 0 {
            lots.pop_front();
        }
    }
    Ok(gain)
}

fn d(s: &str) -> Dec {
    s.parse().unwrap()
}

fn main() {
    for s in ["12.5", "-0.0001", "1,234,567.89", "+3", "", "1.23456", "12a", "99999999999999999999999999999999999999"] {
        match s.parse::<Dec>() {
            Ok(v) => println!("{:>40?} -> {:#.4} | {:.0} | {:+.1}", s, v, v, v),
            Err(e) => println!("{:>40?} -> error {:?}", s, e),
        }
    }
    let x = d("2.345");
    let y = d("-2.345");
    for m in [Rounding::HalfEven, Rounding::HalfUp, Rounding::Down] {
        println!("{:?}: {} {} {} {}", m, x.round_to(2, m), y.round_to(2, m), d("2.355").round_to(2, m), d("0.5").round_to(0, m));
    }
    println!("{} {} {} {:?}", d("10") / 3, d("2") / 3, d("1.0001") * 3, d("1.5").mul_rounded(d("1.5")));
    println!("{:>12}|{:<12}|{:^12}|{:012.2}", d("1234.5"), d("-1234.5"), d("7"), d("-42.125"));
    let parts: Vec<Dec> = (0..7).map(|_| d("100") / 7).collect();
    let total: Dec = parts.iter().sum();
    println!("100/7 x7 = {:.4} (residual {:.4})", total, d("100") - total);

    let mut l = Ledger { accounts: BTreeMap::new(), balances: HashMap::new(), journal: vec![], rates: HashMap::new() };
    for (a, k) in [
        ("assets:bank", Kind::Asset),
        ("assets:brokerage", Kind::Asset),
        ("assets:eur-wallet", Kind::Asset),
        ("liabilities:card", Kind::Liability),
        ("equity:opening", Kind::Equity),
        ("income:salary", Kind::Income),
        ("income:interest", Kind::Income),
        ("income:capital-gains", Kind::Income),
        ("expenses:rent", Kind::Expense),
        ("expenses:food", Kind::Expense),
        ("expenses:travel", Kind::Expense),
    ] {
        l.accounts.insert(a, k);
    }
    l.rates.insert(("EUR", "USD"), d("1.0825"));
    let p = |account: &'static str, amount: &str, currency: &'static str| Posting { account, amount: d(amount), currency };
    let txns = vec![
        Txn { date: (2024, 1, 1), memo: "opening".into(), postings: vec![p("assets:bank", "5000", "USD"), p("equity:opening", "-5000", "USD")] },
        Txn { date: (2024, 1, 3), memo: "rent".into(), postings: vec![p("expenses:rent", "1450", "USD"), p("assets:bank", "-1450", "USD")] },
        Txn { date: (2024, 1, 15), memo: "salary".into(), postings: vec![p("assets:bank", "4210.37", "USD"), p("income:salary", "-4210.37", "USD")] },
        Txn { date: (2024, 1, 16), memo: "groceries on card".into(), postings: vec![p("expenses:food", "87.12", "USD"), p("expenses:food", "12.88", "USD"), p("liabilities:card", "-100", "USD")] },
        Txn { date: (2024, 1, 20), memo: "typo".into(), postings: vec![p("expenses:food", "10", "USD"), p("assets:bank", "-1", "USD")] },
        Txn { date: (2024, 1, 21), memo: "bad account".into(), postings: vec![p("expenses:fun", "10", "USD"), p("assets:bank", "-10", "USD")] },
        Txn { date: (2024, 1, 22), memo: "empty".into(), postings: vec![] },
        Txn { date: (2024, 2, 1), memo: "buy EUR".into(), postings: vec![p("assets:eur-wallet", "500", "EUR"), p("equity:opening", "-500", "EUR"), p("assets:bank", "-541.25", "USD"), p("equity:opening", "541.25", "USD")] },
        Txn { date: (2024, 2, 5), memo: "paris".into(), postings: vec![p("expenses:travel", "312.40", "EUR"), p("assets:eur-wallet", "-312.40", "EUR")] },
        Txn { date: (2024, 2, 10), memo: "pay card".into(), postings: vec![p("liabilities:card", "100", "USD"), p("assets:bank", "-100", "USD")] },
    ];
    for t in txns {
        let memo = t.memo.clone();
        match l.post(t) {
            Ok(()) => {}
            Err(e) => println!("rejected {:?}: {:?}", memo, e),
        }
    }
    // monthly interest accrual on the bank balance, 4.5% APR, daily-ish
    let apr = d("0.045");
    for month in 2..=6u8 {
        let bal = l.balances[&("assets:bank", "USD")];
        let interest = bal.mul_rounded(apr) / 12;
        let interest = interest.round_to(2, Rounding::HalfEven);
        l.post(Txn { date: (2024, month, 28), memo: format!("interest {}", month), postings: vec![p("assets:bank", &format!("{:.4}", interest), "USD"), p("income:interest", &format!("{:.4}", -interest), "USD")] }).unwrap();
    }
    // brokerage with FIFO lots
    let mut lots: VecDeque<Lot> = VecDeque::new();
    let mut realized = Dec::ZERO;
    for (action, qty, price) in [("buy", 10, "101.25"), ("buy", 5, "98.40"), ("sell", 12, "105.00"), ("buy", 8, "110.10"), ("sell", 20, "120"), ("sell", 6, "95.5")] {
        let price = d(price);
        match action {
            "buy" => {
                lots.push_back(Lot { qty, cost: price });
                let cost = price * qty;
                l.post(Txn { date: (2024, 3, 1), memo: format!("buy {}", qty), postings: vec![p("assets:brokerage", &format!("{:.4}", cost), "USD"), p("assets:bank", &format!("{:.4}", -cost), "USD")] }).unwrap();
            }
            _ => match fifo_sell(&mut lots, qty, price) {
                Ok(g) => {
                    realized += g;
                    let proceeds = price * qty;
                    let basis = proceeds - g;
                    l.post(Txn {
                        date: (2024, 4, 1),
                        memo: format!("sell {}", qty),
                        postings: vec![p("assets:bank", &format!("{:.4}", proceeds), "USD"), p("assets:brokerage", &format!("{:.4}", -basis), "USD"), p("income:capital-gains", &format!("{:.4}", -g), "USD")],
                    })
                    .unwrap();
                    println!("sold {} @ {} gain {}", qty, price, g);
                }
                Err(e) => println!("sell failed: {}", e),
            },
        }
    }
    println!("realized {} remaining lots {:?}", realized, lots.iter().map(|l| (l.qty, l.cost)).collect::<Vec<_>>());
    let (lines, dr, cr) = l.trial_balance("USD");
    for line in &lines {
        println!("{}", line);
    }
    println!("debits {:#.2} credits {:#.2} diff {:.4}", dr, cr, dr - cr);
    let by_kind: BTreeMap<Kind, Dec> = l.balances.iter().fold(BTreeMap::new(), |mut m, ((a, c), v)| {
        *m.entry(l.accounts[a]).or_default() += l.convert(*v, c, "USD").unwrap();
        m
    });
    println!("{:?}", by_kind);
    let net_income = -(by_kind[&Kind::Income] + by_kind[&Kind::Expense]);
    println!("net income {:#.2}; journal {} txns, first {:?} last {:?}", net_income, l.journal.len(), l.journal.first().map(|t| t.date), l.journal.last().map(|t| &t.memo));
    println!("EUR->USD {} USD->EUR {}", l.convert(d("100"), "EUR", "USD").unwrap(), l.convert(d("108.25"), "USD", "EUR").unwrap());
    println!("{}", format!("{:#.2}", d("-1234567.891")));
}
