#[derive(Debug, Clone)]
struct Sale {
    region: &'static str,
    product: &'static str,
    units: u32,
    price: f64,
}

fn main() {
    let sales = vec![
        Sale { region: "north", product: "widget", units: 10, price: 2.5 },
        Sale { region: "south", product: "gadget", units: 3, price: 10.0 },
        Sale { region: "north", product: "gadget", units: 7, price: 10.0 },
        Sale { region: "east", product: "widget", units: 22, price: 2.5 },
        Sale { region: "south", product: "widget", units: 5, price: 2.5 },
        Sale { region: "east", product: "gizmo", units: 1, price: 99.99 },
        Sale { region: "north", product: "gizmo", units: 2, price: 99.99 },
    ];
    let mut by_region = sales.clone();
    by_region.sort_by(|a, b| a.region.cmp(b.region).then(b.units.cmp(&a.units)));
    let mut i = 0;
    while i < by_region.len() {
        let region = by_region[i].region;
        let mut total = 0.0;
        println!("{}:", region);
        while i < by_region.len() && by_region[i].region == region {
            let s = &by_region[i];
            let revenue = s.units as f64 * s.price;
            println!("  {:<8} {:>3} x {:>6.2} = {:>8.2}", s.product, s.units, s.price, revenue);
            total += revenue;
            i += 1;
        }
        println!("  total {:.2}", total);
    }
    let mut products: Vec<&str> = sales.iter().map(|s| s.product).collect();
    products.sort();
    products.dedup();
    println!("products: {:?}", products);
    let mut pairs: Vec<(&str, u32)> = products
        .iter()
        .map(|p| (*p, sales.iter().filter(|s| s.product == *p).map(|s| s.units).sum()))
        .collect();
    pairs.sort_by(|a, b| b.1.cmp(&a.1));
    println!("units per product: {:?}", pairs);
    let (big, small): (Vec<&Sale>, Vec<&Sale>) = sales.iter().partition(|s| s.units >= 5);
    println!("big orders: {}, small orders: {}", big.len(), small.len());
    let regions: Vec<(usize, &str)> = sales.iter().map(|s| s.region).enumerate().filter(|(i, _)| i % 2 == 0).collect();
    println!("even-index regions: {:?}", regions);
    let tuples: Vec<(String, u32)> = sales.iter().map(|s| (format!("{}/{}", s.region, s.product), s.units)).collect();
    let (labels, units): (Vec<String>, Vec<u32>) = tuples.into_iter().unzip();
    println!("labels: {:?}", labels);
    println!("units: {:?}", units);
}
