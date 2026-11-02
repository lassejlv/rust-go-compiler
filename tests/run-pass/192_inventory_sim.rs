use std::collections::BTreeMap;
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
enum InvError {
    UnknownSku(String),
    InsufficientStock { sku: String, wanted: u32, available: u32 },
    InsufficientFunds { needed: u64, available: u64 },
    InvalidQuantity,
}

impl fmt::Display for InvError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InvError::UnknownSku(s) => write!(f, "unknown sku {}", s),
            InvError::InsufficientStock { sku, wanted, available } => {
                write!(f, "not enough {}: wanted {}, have {}", sku, wanted, available)
            }
            InvError::InsufficientFunds { needed, available } => {
                write!(f, "insufficient funds: need {}, have {}", Money(*needed), Money(*available))
            }
            InvError::InvalidQuantity => write!(f, "quantity must be positive"),
        }
    }
}

impl std::error::Error for InvError {}

struct Money(u64);
impl fmt::Display for Money {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "${}.{:02}", self.0 / 100, self.0 % 100)
    }
}

trait Priced {
    fn unit_price(&self) -> u64;
    fn discount_pct(&self, qty: u32) -> u64 {
        if qty >= 10 {
            10
        } else {
            0
        }
    }
    fn total(&self, qty: u32) -> u64 {
        let gross = self.unit_price() * qty as u64;
        gross - gross * self.discount_pct(qty) / 100
    }
}

#[derive(Debug, Clone)]
struct Product {
    name: String,
    price_cents: u64,
    stock: u32,
    bulk: bool,
}

impl Priced for Product {
    fn unit_price(&self) -> u64 {
        self.price_cents
    }
    fn discount_pct(&self, qty: u32) -> u64 {
        match (self.bulk, qty) {
            (true, q) if q >= 20 => 25,
            (true, q) if q >= 5 => 15,
            (_, q) if q >= 10 => 10,
            _ => 0,
        }
    }
}

#[derive(Debug)]
enum Event {
    Restock(&'static str, u32),
    Order { customer: &'static str, sku: &'static str, qty: u32 },
    Deposit(&'static str, u64),
}

struct Store {
    products: BTreeMap<String, Product>,
    wallets: BTreeMap<String, u64>,
    revenue: u64,
    log: Vec<String>,
}

impl Store {
    fn apply(&mut self, e: &Event) -> Result<String, InvError> {
        match *e {
            Event::Restock(sku, qty) => {
                let p = self.products.get_mut(sku).ok_or_else(|| InvError::UnknownSku(sku.to_string()))?;
                p.stock += qty;
                Ok(format!("restocked {} to {}", sku, p.stock))
            }
            Event::Deposit(who, amt) => {
                let w = self.wallets.entry(who.to_string()).or_insert(0);
                *w += amt;
                Ok(format!("{} balance {}", who, Money(*w)))
            }
            Event::Order { customer, sku, qty } => {
                if qty == 0 {
                    return Err(InvError::InvalidQuantity);
                }
                let p = self.products.get(sku).ok_or_else(|| InvError::UnknownSku(sku.to_string()))?;
                if p.stock < qty {
                    return Err(InvError::InsufficientStock { sku: sku.to_string(), wanted: qty, available: p.stock });
                }
                let cost = p.total(qty);
                let (name, pct) = (p.name.clone(), p.discount_pct(qty));
                let wallet = self.wallets.get(customer).copied().unwrap_or(0);
                if wallet < cost {
                    return Err(InvError::InsufficientFunds { needed: cost, available: wallet });
                }
                self.wallets.insert(customer.to_string(), wallet - cost);
                self.products.get_mut(sku).unwrap().stock -= qty;
                self.revenue += cost;
                Ok(format!("{} bought {} x {} for {} ({}% off)", customer, qty, name, Money(cost), pct))
            }
        }
    }
}

fn main() {
    let mut store = Store { products: BTreeMap::new(), wallets: BTreeMap::new(), revenue: 0, log: vec![] };
    for (sku, name, price, stock, bulk) in [("W1", "widget", 250u64, 30u32, true), ("G7", "gadget", 1999, 3, false), ("S3", "sprocket", 75, 100, true)] {
        store.products.insert(sku.to_string(), Product { name: name.to_string(), price_cents: price, stock, bulk });
    }
    let events = vec![
        Event::Deposit("ann", 10_000),
        Event::Deposit("bob", 2_500),
        Event::Order { customer: "ann", sku: "W1", qty: 4 },
        Event::Order { customer: "ann", sku: "W1", qty: 5 },
        Event::Order { customer: "bob", sku: "G7", qty: 2 },
        Event::Order { customer: "bob", sku: "G7", qty: 1 },
        Event::Order { customer: "ann", sku: "G7", qty: 3 },
        Event::Order { customer: "ann", sku: "S3", qty: 20 },
        Event::Order { customer: "carl", sku: "S3", qty: 1 },
        Event::Order { customer: "ann", sku: "XX", qty: 1 },
        Event::Order { customer: "ann", sku: "S3", qty: 0 },
        Event::Restock("G7", 10),
        Event::Restock("ZZ", 1),
        Event::Order { customer: "ann", sku: "G7", qty: 2 },
    ];
    let mut errors = 0;
    for (i, e) in events.iter().enumerate() {
        match store.apply(e) {
            Ok(msg) => store.log.push(format!("#{:02} ok  {}", i, msg)),
            Err(err) => {
                errors += 1;
                store.log.push(format!("#{:02} ERR {}", i, err))
            }
        }
    }
    for l in &store.log {
        println!("{}", l);
    }
    println!("errors: {}", errors);
    println!("revenue: {}", Money(store.revenue));
    for (sku, p) in &store.products {
        println!("{} {:<9} stock {:>3} value {}", sku, p.name, p.stock, Money(p.price_cents * p.stock as u64));
    }
    for (who, amt) in &store.wallets {
        println!("{} has {}", who, Money(*amt));
    }
    let boxed: Box<dyn std::error::Error> = Box::new(InvError::InvalidQuantity);
    println!("boxed: {}", boxed);
}
