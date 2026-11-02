use std::collections::HashMap;

#[derive(Debug, Clone)]
struct Product {
    name: String,
    price_cents: u64,
    qty: u32,
}

struct Store {
    products: HashMap<String, Product>,
    orders: HashMap<u32, Vec<(String, u32)>>,
    next_order: u32,
}

impl Store {
    fn new() -> Self {
        Store { products: HashMap::new(), orders: HashMap::new(), next_order: 1 }
    }

    fn add_product(&mut self, sku: &str, name: &str, price_cents: u64, qty: u32) {
        self.products.insert(sku.to_string(), Product { name: name.to_string(), price_cents, qty });
    }

    fn order(&mut self, items: &[(&str, u32)]) -> Result<u32, String> {
        for (sku, n) in items {
            let p = self.products.get(*sku).ok_or(format!("unknown sku {}", sku))?;
            if p.qty < *n {
                return Err(format!("not enough {} (have {}, want {})", p.name, p.qty, n));
            }
        }
        for (sku, n) in items {
            if let Some(p) = self.products.get_mut(*sku) {
                p.qty -= n;
            }
        }
        let id = self.next_order;
        self.next_order += 1;
        self.orders.insert(id, items.iter().map(|(s, n)| (s.to_string(), *n)).collect());
        Ok(id)
    }

    fn order_total(&self, id: u32) -> Option<u64> {
        let items = self.orders.get(&id)?;
        Some(items.iter().map(|(sku, n)| self.products[sku].price_cents * *n as u64).sum())
    }

    fn report(&self) {
        let mut skus: Vec<&String> = self.products.keys().collect();
        skus.sort();
        for s in skus {
            let p = &self.products[s];
            println!("  {:<6} {:<12} ${:>3}.{:02} x{}", s, p.name, p.price_cents / 100, p.price_cents % 100, p.qty);
        }
    }
}

fn main() {
    let mut store = Store::new();
    store.add_product("A100", "keyboard", 4999, 10);
    store.add_product("B200", "mouse", 1950, 3);
    store.add_product("C300", "monitor", 18900, 2);
    store.report();
    let attempts: Vec<Vec<(&str, u32)>> = vec![
        vec![("A100", 2), ("B200", 1)],
        vec![("C300", 3)],
        vec![("Z999", 1)],
        vec![("B200", 2), ("C300", 1)],
        vec![("B200", 1)],
    ];
    for items in &attempts {
        match store.order(items) {
            Ok(id) => println!("order #{} ok, total ${:.2}", id, store.order_total(id).unwrap() as f64 / 100.0),
            Err(e) => println!("order failed: {}", e),
        }
    }
    store.report();
    println!("order 99 total: {:?}", store.order_total(99));
    let mut ids: Vec<&u32> = store.orders.keys().collect();
    ids.sort();
    println!("order ids: {:?}", ids);
    let mut sold: HashMap<&str, u32> = HashMap::new();
    for items in store.orders.values() {
        for (sku, n) in items {
            *sold.entry(sku.as_str()).or_insert(0) += n;
        }
    }
    let mut sold_v: Vec<_> = sold.into_iter().collect();
    sold_v.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));
    println!("units sold: {:?}", sold_v);
    let inventory_value: u64 = store.products.values().map(|p| p.price_cents * p.qty as u64).sum();
    println!("inventory value: ${}.{:02}", inventory_value / 100, inventory_value % 100);
}
