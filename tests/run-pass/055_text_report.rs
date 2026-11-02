struct Employee {
    name: &'static str,
    dept: &'static str,
    salary: u32,
    rating: f64,
}

fn bar(value: f64, max: f64, width: usize) -> String {
    let filled = ((value / max) * width as f64).round() as usize;
    format!("{}{}", "#".repeat(filled), ".".repeat(width - filled))
}

fn main() {
    let staff = [
        Employee { name: "Alice", dept: "Engineering", salary: 125000, rating: 4.8 },
        Employee { name: "Bob", dept: "Marketing", salary: 72000, rating: 3.9 },
        Employee { name: "Charlotte", dept: "Engineering", salary: 98000, rating: 4.25 },
        Employee { name: "Dan", dept: "Sales", salary: 61000, rating: 2.5 },
        Employee { name: "Eve", dept: "Sales", salary: 88500, rating: 4.0 },
    ];
    let header = format!("{:<10} {:<12} {:>10} {:>6}  {}", "Name", "Department", "Salary", "Rating", "Bar");
    println!("{}", header);
    println!("{}", "=".repeat(header.len() + 6));
    let mut total = 0u32;
    for e in staff.iter() {
        total += e.salary;
        println!(
            "{:<10} {:<12} {:>10} {:>6.2}  {}",
            e.name,
            e.dept,
            format!("${}", e.salary),
            e.rating,
            bar(e.rating, 5.0, 10)
        );
    }
    println!("{}", "-".repeat(header.len() + 6));
    println!("{:<23} {:>10}", "Total", format!("${}", total));
    println!("{:<23} {:>10.1}", "Average", total as f64 / staff.len() as f64);
    let depts = ["Engineering", "Marketing", "Sales"];
    println!();
    println!("{:^30}", "Per department");
    for d in depts {
        let members: Vec<&Employee> = staff.iter().filter(|e| e.dept == d).collect();
        let sum: u32 = members.iter().map(|e| e.salary).sum();
        let names: Vec<&str> = members.iter().map(|e| e.name).collect();
        println!("{:.<15}{:.>15}", d, sum);
        println!("    members: {}", names.join(", "));
    }
    let best = staff.iter().max_by(|a, b| a.rating.partial_cmp(&b.rating).unwrap()).unwrap();
    println!("\nTop performer: {} ({:.1})", best.name, best.rating);
    println!("Percentages:");
    for e in &staff {
        println!("  {:>9}: {:>5.1}%", e.name, e.salary as f64 * 100.0 / total as f64);
    }
}
