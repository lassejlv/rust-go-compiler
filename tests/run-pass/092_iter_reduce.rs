#[derive(Debug)]
struct City {
    name: &'static str,
    pop: u32,
    area: f64,
}

fn main() {
    let v = [4, -2, 9, 0, 9, 3, -7];
    println!("sum = {}", v.iter().sum::<i32>());
    println!("product = {}", v.iter().filter(|&&x| x != 0).product::<i32>());
    println!("count = {}", v.iter().count());
    println!("count positive = {}", v.iter().filter(|x| **x > 0).count());
    println!("min = {:?}, max = {:?}", v.iter().min(), v.iter().max());
    println!("max position = {:?}", v.iter().position(|&x| x == 9));
    println!("rposition = {:?}", v.iter().rposition(|&x| x == 9));
    println!("any negative = {}", v.iter().any(|&x| x < 0));
    println!("all < 10 = {}", v.iter().all(|&x| x < 10));
    println!("all on empty = {}", Vec::<i32>::new().iter().all(|&x| x > 100));
    println!("find > 5 = {:?}", v.iter().find(|&&x| x > 5));
    println!("find_map = {:?}", v.iter().find_map(|&x| if x < 0 { Some(x * 100) } else { None }));
    println!("last = {:?}", v.iter().last());
    println!("nth(2) = {:?}, nth(20) = {:?}", v.iter().nth(2), v.iter().nth(20));
    println!("min_by_key abs = {:?}", v.iter().min_by_key(|x| x.abs()));
    println!("max_by_key abs = {:?}", v.iter().max_by_key(|x| x.abs()));
    let cities = vec![
        City { name: "Avalon", pop: 120000, area: 50.5 },
        City { name: "Brook", pop: 45000, area: 12.0 },
        City { name: "Cedar", pop: 980000, area: 310.25 },
        City { name: "Dune", pop: 45000, area: 99.0 },
    ];
    let biggest = cities.iter().max_by_key(|c| c.pop).unwrap();
    println!("biggest = {}", biggest.name);
    let smallest = cities.iter().min_by_key(|c| c.pop).unwrap();
    println!("smallest (first of ties) = {}", smallest.name);
    let largest_area = cities.iter().max_by(|a, b| a.area.partial_cmp(&b.area).unwrap()).unwrap();
    println!("largest area = {}", largest_area.name);
    let densest = cities
        .iter()
        .max_by(|a, b| (a.pop as f64 / a.area).partial_cmp(&(b.pop as f64 / b.area)).unwrap())
        .unwrap();
    println!("densest = {} ({:.1}/km2)", densest.name, densest.pop as f64 / densest.area);
    let total_pop: u32 = cities.iter().map(|c| c.pop).sum();
    println!("total pop = {}", total_pop);
    let total_area: f64 = cities.iter().map(|c| c.area).sum();
    println!("total area = {}", total_area);
    let fsum: f64 = [0.1, 0.2, 0.3].iter().sum();
    println!("float sum = {}", fsum);
    let fprod: f64 = [1.5, 2.0, 4.0].iter().product();
    println!("float product = {}", fprod);
    let fmax = [1.5, -2.0, 8.25, 3.0].iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    println!("float max = {}", fmax);
    let empty: Vec<i32> = Vec::new();
    println!("empty: sum={} max={:?} count={}", empty.iter().sum::<i32>(), empty.iter().max(), empty.iter().count());
    let eq = [1, 2, 3].iter().eq([1, 2, 3].iter());
    let lt = [1, 2].iter().lt([1, 3].iter());
    println!("iter eq = {}, iter lt = {}", eq, lt);
    let is_sorted = v.windows(2).all(|w| w[0] <= w[1]);
    println!("is sorted = {}", is_sorted);
}
