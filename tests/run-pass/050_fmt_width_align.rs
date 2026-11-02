fn main() {
    println!("[{:5}]", 42);
    println!("[{:<5}]", 42);
    println!("[{:^5}]", 42);
    println!("[{:>5}]", 42);
    println!("[{:5}]", "ab");
    println!("[{:>5}]", "ab");
    println!("[{:^6}]", "ab");
    println!("[{:^7}]", "ab");
    println!("[{:*<8}]", "left");
    println!("[{:->8}]", "right");
    println!("[{:#^9}]", "mid");
    println!("[{:05}]", 42);
    println!("[{:05}]", -42);
    println!("[{:<05}]", 7);
    println!("[{:2}]", 12345);
    println!("[{:3}]", "toolong");
    println!("[{:.3}]", "truncate");
    println!("[{:8.3}]", "truncate");
    println!("[{:>8.2}]", 3.14159);
    println!("[{:<8.2}]", 3.14159);
    println!("[{:^8.1}]", 3.14159);
    println!("[{:08.3}]", 3.14159);
    println!("[{:08.3}]", -3.14159);
    println!("[{:+08.2}]", 2.5);
    let width = 10;
    let prec = 4;
    println!("[{:>width$}]", "dyn");
    println!("[{:>1$}]", "pos", 7);
    println!("[{:width$.prec$}]", 2.0f64.sqrt());
    println!("[{:.*}]", 2, 1.23456);
    println!("[{:>w$.p$}]", 9.87654, w = 9, p = 1);
    println!("[{:5}]", true);
    println!("[{:<6}]", 'c');
    println!("[{:>4}]", "é");
    println!("[{:e<4}]", "");
    let items = [("apple", 3, 0.5), ("banana", 12, 0.25), ("cherry", 150, 10.0)];
    println!("{:<10}|{:>5}|{:>8}", "item", "qty", "price");
    println!("{:-<10}+{:->5}+{:->8}", "", "", "");
    for (name, qty, price) in items.iter() {
        println!("{:<10}|{:>5}|{:>8.2}", name, qty, price);
    }
}
