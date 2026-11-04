// Inventory / order fulfilment system with HashMap, BTreeMap, sorting and reports.
use std::cmp::{Ordering, Reverse};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
struct Sku(String);

#[derive(Debug, Clone)]
struct Product {
    sku: Sku,
    name: String,
    category: Category,
    price_cents: u64,
    weight_g: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
enum Category {
    Electronics,
    Grocery,
    Toys,
    Books,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
enum Warehouse {
    North,
    South,
    West,
}

#[derive(Debug, Clone)]
struct OrderLine {
    sku: Sku,
    qty: u32,
}

#[derive(Debug, Clone)]
struct Order {
    id: u32,
    customer: String,
    priority: u8,
    lines: Vec<OrderLine>,
    coupon: Option<&'static str>,
}

#[derive(Debug, PartialEq)]
enum Fulfilment {
    Shipped { from: Vec<(Warehouse, Sku, u32)>, total: u64, shipping: u64 },
    Partial { shipped: u32, backordered: Vec<(Sku, u32)> },
    Rejected(String),
}

impl fmt::Display for Fulfilment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Fulfilment::Shipped { from, total, shipping } => {
                let ws: HashSet<Warehouse> = from.iter().map(|x| x.0).collect();
                let mut ws: Vec<_> = ws.into_iter().collect();
                ws.sort();
                write!(f, "SHIPPED {} picks from {:?}, total {}, shipping {}", from.len(), ws, money(*total), money(*shipping))
            }
            Fulfilment::Partial { shipped, backordered } => {
                write!(f, "PARTIAL shipped {} units, backordered ", shipped)?;
                let parts: Vec<String> = backordered.iter().map(|(s, q)| format!("{}x{}", s.0, q)).collect();
                write!(f, "{}", parts.join(","))
            }
            Fulfilment::Rejected(why) => write!(f, "REJECTED: {}", why),
        }
    }
}

fn money(c: u64) -> String {
    format!("${}.{:02}", c / 100, c % 100)
}

struct Inventory {
    catalog: HashMap<Sku, Product>,
    stock: BTreeMap<(Warehouse, Sku), u32>,
    reserved: HashMap<Sku, u32>,
    sales: BTreeMap<Category, (u32, u64)>,
    backorders: Vec<(u32, Sku, u32)>,
}

impl Inventory {
    fn available(&self, sku: &Sku) -> u32 {
        self.stock.iter().filter(|((_, s), _)| s == sku).map(|(_, q)| *q).sum::<u32>()
            - self.reserved.get(sku).copied().unwrap_or(0)
    }

    fn pick(&mut self, sku: &Sku, mut qty: u32) -> Vec<(Warehouse, Sku, u32)> {
        // pick from the warehouse with the most stock first, ties by warehouse order
        let mut sources: Vec<(Warehouse, u32)> = self
            .stock
            .range((Warehouse::North, sku.clone())..)
            .filter(|((_, s), q)| s == sku && **q > 0)
            .map(|((w, _), q)| (*w, *q))
            .collect();
        // range above starts at North but stock keys are (Warehouse, Sku) - collect others too
        for w in [Warehouse::South, Warehouse::West] {
            if let Some(&q) = self.stock.get(&(w, sku.clone())) {
                if q > 0 && !sources.iter().any(|(x, _)| *x == w) {
                    sources.push((w, q));
                }
            }
        }
        sources.sort_by_key(|&(w, q)| (Reverse(q), w));
        let mut picks = Vec::new();
        for (w, have) in sources {
            if qty == 0 {
                break;
            }
            let take = have.min(qty);
            *self.stock.get_mut(&(w, sku.clone())).unwrap() -= take;
            qty -= take;
            picks.push((w, sku.clone(), take));
        }
        picks
    }

    fn process(&mut self, o: &Order) -> Fulfilment {
        if o.lines.is_empty() {
            return Fulfilment::Rejected("empty order".into());
        }
        for l in &o.lines {
            if !self.catalog.contains_key(&l.sku) {
                return Fulfilment::Rejected(format!("unknown sku {}", l.sku.0));
            }
            if l.qty == 0 {
                return Fulfilment::Rejected(format!("zero quantity for {}", l.sku.0));
            }
        }
        // merge duplicate lines
        let mut merged: BTreeMap<Sku, u32> = BTreeMap::new();
        for l in &o.lines {
            *merged.entry(l.sku.clone()).or_insert(0) += l.qty;
        }
        let short: Vec<(Sku, u32)> = merged
            .iter()
            .filter_map(|(s, &q)| {
                let a = self.available(s);
                if a < q { Some((s.clone(), q - a)) } else { None }
            })
            .collect();
        if !short.is_empty() {
            let mut shipped = 0;
            for (s, q) in &merged {
                let missing = short.iter().find(|(x, _)| x == s).map(|x| x.1).unwrap_or(0);
                let can = q - missing;
                if can > 0 {
                    let picks = self.pick(s, can);
                    shipped += picks.iter().map(|p| p.2).sum::<u32>();
                    self.account(s, can);
                }
                if missing > 0 {
                    self.backorders.push((o.id, s.clone(), missing));
                }
            }
            return Fulfilment::Partial { shipped, backordered: short };
        }
        let mut from = Vec::new();
        let mut subtotal = 0u64;
        let mut weight = 0u64;
        for (s, q) in &merged {
            from.extend(self.pick(s, *q));
            let p = &self.catalog[s];
            subtotal += p.price_cents * *q as u64;
            weight += p.weight_g as u64 * *q as u64;
            self.account(s, *q);
        }
        let discount = match o.coupon {
            Some("TENOFF") if subtotal >= 5000 => subtotal / 10,
            Some("FREESHIP") => 0,
            Some(c) if c.starts_with("FLAT") => c[4..].parse::<u64>().unwrap_or(0) * 100,
            _ => 0,
        };
        let warehouses: HashSet<_> = from.iter().map(|p| p.0).collect();
        let mut shipping = 499 + (weight / 500) * 150 + (warehouses.len() as u64 - 1) * 300;
        if o.coupon == Some("FREESHIP") || o.priority == 0 {
            shipping = 0;
        }
        Fulfilment::Shipped { from, total: subtotal.saturating_sub(discount) + shipping, shipping }
    }

    fn account(&mut self, s: &Sku, q: u32) {
        let p = &self.catalog[s];
        let e = self.sales.entry(p.category).or_insert((0, 0));
        e.0 += q;
        e.1 += p.price_cents * q as u64;
    }
}

fn main() {
    let raw = [
        ("E-100", "USB cable", Category::Electronics, 899, 40),
        ("E-200", "Headphones", Category::Electronics, 5999, 250),
        ("E-300", "Monitor", Category::Electronics, 18999, 4200),
        ("G-001", "Coffee beans", Category::Grocery, 1450, 1000),
        ("G-002", "Tea", Category::Grocery, 650, 200),
        ("T-010", "Puzzle", Category::Toys, 2200, 700),
        ("T-020", "Kite", Category::Toys, 1799, 300),
        ("B-777", "Rust book", Category::Books, 3995, 900),
        ("B-778", "Go book", Category::Books, 3495, 850),
    ];
    let catalog: HashMap<Sku, Product> = raw
        .iter()
        .map(|&(s, n, c, p, w)| (Sku(s.to_string()), Product { sku: Sku(s.to_string()), name: n.to_string(), category: c, price_cents: p, weight_g: w }))
        .collect();
    let mut stock = BTreeMap::new();
    let mut seed = 7u32;
    let mut skus_owned: Vec<Sku> = catalog.keys().cloned().collect();
    skus_owned.sort();
    let skus: Vec<&Sku> = skus_owned.iter().collect();
    for s in &skus {
        for w in [Warehouse::North, Warehouse::South, Warehouse::West] {
            seed = seed.wrapping_mul(1103515245).wrapping_add(12345);
            let q = (seed >> 16) % 9;
            stock.insert((w, (*s).clone()), q);
        }
    }
    let mut inv = Inventory { catalog, stock, reserved: HashMap::new(), sales: BTreeMap::new(), backorders: vec![] };

    println!("Initial stock:");
    for s in &skus {
        let per: Vec<String> = [Warehouse::North, Warehouse::South, Warehouse::West]
            .iter()
            .map(|w| format!("{:>2}", inv.stock[&(*w, (*s).clone())]))
            .collect();
        println!("  {:<6} {:<14} {} = {:>2}", s.0, inv.catalog[*s].name, per.join(" "), inv.available(s));
    }

    let line = |s: &str, q: u32| OrderLine { sku: Sku(s.into()), qty: q };
    let mut orders = vec![
        Order { id: 1, customer: "zed".into(), priority: 2, lines: vec![line("E-100", 3), line("G-002", 2)], coupon: None },
        Order { id: 2, customer: "amy".into(), priority: 1, lines: vec![line("E-300", 1), line("B-777", 1)], coupon: Some("TENOFF") },
        Order { id: 3, customer: "bob".into(), priority: 3, lines: vec![line("T-010", 30)], coupon: None },
        Order { id: 4, customer: "amy".into(), priority: 0, lines: vec![line("G-001", 2), line("G-001", 1)], coupon: Some("FLAT5") },
        Order { id: 5, customer: "cat".into(), priority: 1, lines: vec![], coupon: None },
        Order { id: 6, customer: "dan".into(), priority: 2, lines: vec![line("X-999", 1)], coupon: None },
        Order { id: 7, customer: "eve".into(), priority: 1, lines: vec![line("E-200", 2), line("T-020", 4)], coupon: Some("FREESHIP") },
        Order { id: 8, customer: "bob".into(), priority: 2, lines: vec![line("B-778", 0)], coupon: None },
        Order { id: 9, customer: "fay".into(), priority: 1, lines: vec![line("E-100", 50), line("G-002", 1)], coupon: None },
        Order { id: 10, customer: "zed".into(), priority: 1, lines: vec![line("B-778", 2), line("B-777", 1), line("E-200", 1)], coupon: Some("TENOFF") },
    ];
    // process by priority (0 = highest), then id
    orders.sort_by(|a, b| a.priority.cmp(&b.priority).then_with(|| a.id.cmp(&b.id)));
    let mut results: Vec<(u32, String, Fulfilment)> = Vec::new();
    for o in &orders {
        let r = inv.process(o);
        println!("order #{:<2} p{} {:<4} {}", o.id, o.priority, o.customer, r);
        results.push((o.id, o.customer.clone(), r));
    }

    println!("\nRevenue by customer:");
    let mut rev: HashMap<&str, u64> = HashMap::new();
    for (_, c, r) in &results {
        if let Fulfilment::Shipped { total, .. } = r {
            *rev.entry(c.as_str()).or_default() += total;
        }
    }
    let mut rv: Vec<(&str, u64)> = rev.into_iter().collect();
    rv.sort_unstable_by(|a, b| match b.1.cmp(&a.1) {
        Ordering::Equal => a.0.cmp(b.0),
        o => o,
    });
    for (c, t) in &rv {
        println!("  {:<5}{:>10}", c, money(*t));
    }

    println!("\nSales by category:");
    for (cat, (units, cents)) in &inv.sales {
        println!("  {:<12} {:>3} units {:>10}", format!("{:?}", cat), units, money(*cents));
    }

    println!("\nBackorders:");
    let mut bo = inv.backorders.clone();
    bo.sort_by(|a, b| b.2.cmp(&a.2).then(a.1.cmp(&b.1)));
    for (oid, s, q) in &bo {
        println!("  order {} needs {} more of {}", oid, q, s.0);
    }

    println!("\nRemaining stock (non-zero), grouped by warehouse:");
    let mut by_w: BTreeMap<Warehouse, Vec<(String, u32)>> = BTreeMap::new();
    for ((w, s), q) in &inv.stock {
        if *q > 0 {
            by_w.entry(*w).or_default().push((s.0.clone(), *q));
        }
    }
    for (w, list) in &by_w {
        let total: u32 = list.iter().map(|x| x.1).sum();
        let s: Vec<String> = list.iter().map(|(s, q)| format!("{}:{}", s, q)).collect();
        println!("  {:?} ({} units): {}", w, total, s.join(" "));
    }
    let empty: Vec<&Sku> = skus.iter().copied().filter(|s| inv.available(s) == 0).collect();
    println!("sold out: {:?}", empty.iter().map(|s| &s.0).collect::<Vec<_>>());
    let mut by_value: Vec<(&Sku, u64)> = skus.iter().map(|s| (*s, inv.available(s) as u64 * inv.catalog[*s].price_cents)).collect();
    by_value.sort_by_key(|&(s, v)| (Reverse(v), s.clone()));
    println!("top inventory value: {:?}", by_value.iter().take(3).map(|(s, v)| format!("{}={}", s.0, money(*v))).collect::<Vec<_>>());
    let heaviest = inv.catalog.values().max_by(|a, b| a.weight_g.cmp(&b.weight_g).then_with(|| b.sku.cmp(&a.sku))).unwrap();
    println!("heaviest product: {} ({}g, {})", heaviest.name, heaviest.weight_g, heaviest.sku.0);
}
