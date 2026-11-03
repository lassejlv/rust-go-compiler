// Integration program: an inventory/order system driven by a line-based
// command DSL. Uses traits with dyn pricing strategies, error enums with
// From, BTreeMap-backed stock with reservations, order state machine,
// fixed-point money formatting, and a final report.
use std::collections::BTreeMap;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct Cents(i64);
impl fmt::Display for Cents {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let s = format!("{}${}.{:02}", if self.0 < 0 { "-" } else { "" }, self.0.abs() / 100, self.0.abs() % 100);
        f.pad(&s)
    }
}
impl std::ops::Add for Cents { type Output = Cents; fn add(self, o: Cents) -> Cents { Cents(self.0 + o.0) } }
impl std::iter::Sum for Cents { fn sum<I: Iterator<Item = Cents>>(i: I) -> Cents { i.fold(Cents(0), |a, b| a + b) } }

#[derive(Debug)]
enum Error { UnknownSku(String), Insufficient { sku: String, want: u32, have: u32 }, BadCommand(String), Parse(std::num::ParseIntError), BadState { order: u32, state: OrderState, action: &'static str }, NoSuchOrder(u32) }
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::UnknownSku(s) => write!(f, "unknown sku {}", s),
            Error::Insufficient { sku, want, have } => write!(f, "insufficient {}: want {} have {}", sku, want, have),
            Error::BadCommand(c) => write!(f, "bad command: {}", c),
            Error::Parse(e) => write!(f, "parse error: {}", e),
            Error::BadState { order, state, action } => write!(f, "order {} is {:?}, cannot {}", order, state, action),
            Error::NoSuchOrder(o) => write!(f, "no order {}", o),
        }
    }
}
impl From<std::num::ParseIntError> for Error { fn from(e: std::num::ParseIntError) -> Self { Error::Parse(e) } }

trait Pricing { fn price(&self, unit: Cents, qty: u32) -> Cents; fn name(&self) -> String; }
struct Flat;
struct Bulk { min: u32, pct_off: i64 }
struct BuyNGetOne(u32);
impl Pricing for Flat { fn price(&self, u: Cents, q: u32) -> Cents { Cents(u.0 * q as i64) } fn name(&self) -> String { "flat".into() } }
impl Pricing for Bulk {
    fn price(&self, u: Cents, q: u32) -> Cents { let base = u.0 * q as i64; Cents(if q >= self.min { base - base * self.pct_off / 100 } else { base }) }
    fn name(&self) -> String { format!("bulk({}+ -{}%)", self.min, self.pct_off) }
}
impl Pricing for BuyNGetOne {
    fn price(&self, u: Cents, q: u32) -> Cents { let free = q / (self.0 + 1); Cents(u.0 * (q - free) as i64) }
    fn name(&self) -> String { format!("buy{}get1", self.0) }
}

struct Item { name: String, unit: Cents, stock: u32, reserved: u32, pricing: Box<dyn Pricing> }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OrderState { Open, Paid, Shipped, Cancelled }

struct Order { id: u32, customer: String, lines: Vec<(String, u32, Cents)>, state: OrderState }
impl Order { fn total(&self) -> Cents { self.lines.iter().map(|l| l.2).sum() } }

#[derive(Default)]
struct Store { items: BTreeMap<String, Item>, orders: BTreeMap<u32, Order>, next_id: u32, revenue: Cents }

impl Default for Cents { fn default() -> Self { Cents(0) } }

impl Store {
    fn exec(&mut self, line: &str) -> Result<String, Error> {
        let w: Vec<&str> = line.split_whitespace().collect();
        match w.as_slice() {
            ["add", sku, name, price, qty, rest @ ..] => {
                let pricing: Box<dyn Pricing> = match rest {
                    [] => Box::new(Flat),
                    ["bulk", min, pct] => Box::new(Bulk { min: min.parse()?, pct_off: pct.parse()? }),
                    ["bogo", n] => Box::new(BuyNGetOne(n.parse()?)),
                    _ => return Err(Error::BadCommand(line.into())),
                };
                let p: i64 = price.replace('.', "").parse()?;
                let pn = pricing.name();
                self.items.insert(sku.to_string(), Item { name: name.replace('_', " "), unit: Cents(p), stock: qty.parse()?, reserved: 0, pricing });
                Ok(format!("added {} ({})", sku, pn))
            }
            ["restock", sku, qty] => {
                let it = self.items.get_mut(*sku).ok_or_else(|| Error::UnknownSku(sku.to_string()))?;
                it.stock += qty.parse::<u32>()?;
                Ok(format!("{} stock {}", sku, it.stock))
            }
            ["order", customer, lines @ ..] => {
                let mut parsed = vec![];
                for l in lines {
                    let (sku, q) = l.split_once('x').ok_or_else(|| Error::BadCommand(l.to_string()))?;
                    let q: u32 = q.parse()?;
                    let it = self.items.get(sku).ok_or_else(|| Error::UnknownSku(sku.into()))?;
                    let avail = it.stock - it.reserved;
                    if q > avail { return Err(Error::Insufficient { sku: sku.into(), want: q, have: avail }); }
                    parsed.push((sku.to_string(), q, it.pricing.price(it.unit, q)));
                }
                for (sku, q, _) in &parsed { self.items.get_mut(sku).unwrap().reserved += q; }
                self.next_id += 1;
                let o = Order { id: self.next_id, customer: customer.to_string(), lines: parsed, state: OrderState::Open };
                let msg = format!("order {} for {} total {}", o.id, o.customer, o.total());
                self.orders.insert(o.id, o);
                Ok(msg)
            }
            [action @ ("pay" | "ship" | "cancel"), id] => {
                let id: u32 = id.parse()?;
                let o = self.orders.get_mut(&id).ok_or(Error::NoSuchOrder(id))?;
                let (from, to, act): (OrderState, OrderState, &'static str) = match *action {
                    "pay" => (OrderState::Open, OrderState::Paid, "pay"),
                    "ship" => (OrderState::Paid, OrderState::Shipped, "ship"),
                    _ => (o.state, OrderState::Cancelled, "cancel"),
                };
                if o.state != from || o.state == OrderState::Shipped || o.state == OrderState::Cancelled {
                    return Err(Error::BadState { order: id, state: o.state, action: act });
                }
                o.state = to;
                for (sku, q, _) in &o.lines {
                    let it = self.items.get_mut(sku).unwrap();
                    match to {
                        OrderState::Shipped => { it.reserved -= q; it.stock -= q; }
                        OrderState::Cancelled => it.reserved -= q,
                        _ => {}
                    }
                }
                if to == OrderState::Paid { self.revenue = self.revenue + o.total(); }
                if to == OrderState::Cancelled && from == OrderState::Paid { self.revenue = Cents(self.revenue.0 - o.total().0); }
                Ok(format!("order {} -> {:?}", id, to))
            }
            [] => Err(Error::BadCommand("<empty>".into())),
            _ => Err(Error::BadCommand(line.into())),
        }
    }
}

const SCRIPT: &str = "
add W1 widget 2.50 100
add G7 deluxe_gadget 19.99 10 bulk 5 10
add S3 sock 3.00 30 bogo 2
order alice W1x10 S3x6
order bob G7x5 W1x1
order carol G7x6
restock G7 4
order carol G7x6
pay 1
ship 1
pay 2
cancel 2
ship 2
order dave X9x1
order erin W1xx
pay 99
frobnicate
add Z9 thing 1.00 5 weird
order frank S3x24 W1x89
pay 3
ship 3
cancel 3
pay 5
";

fn main() {
    let mut st = Store::default();
    for (n, line) in SCRIPT.lines().map(str::trim).filter(|l| !l.is_empty()).enumerate() {
        match st.exec(line) {
            Ok(m) => println!("{:>2} ok   {}", n + 1, m),
            Err(e) => println!("{:>2} ERR  {} [{:?}]", n + 1, e, std::mem::discriminant(&e) == std::mem::discriminant(&Error::NoSuchOrder(0))),
        }
    }
    println!("{:-<60}", "");
    println!("{:<4}|{:<16}|{:>9}|{:>6}|{:>8}", "sku", "name", "unit", "stock", "reserved");
    for (sku, it) in &st.items { println!("{:<4}|{:<16}|{:>9}|{:>6}|{:>8}", sku, it.name, it.unit, it.stock, it.reserved); }
    println!("{:-<60}", "");
    for o in st.orders.values() {
        let lines: Vec<String> = o.lines.iter().map(|(s, q, c)| format!("{}x{}={}", s, q, c)).collect();
        println!("#{} {:<6} {:<10} {:>9}  {}", o.id, o.customer, format!("{:?}", o.state), o.total(), lines.join(", "));
    }
    let by_state = st.orders.values().fold(BTreeMap::new(), |mut m, o| { *m.entry(format!("{:?}", o.state)).or_insert(0) += 1; m });
    println!("revenue {} | orders by state {:?}", st.revenue, by_state);
    println!("[{:>12}] [{:<12}] [{:^12}]", Cents(-1234), Cents(5), Cents(100000));
}
