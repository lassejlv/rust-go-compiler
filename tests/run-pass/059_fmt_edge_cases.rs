#[derive(Debug)]
struct P {
    x: i32,
    y: f64,
}

fn main() {
    println!("[{:>6?}]", Some(1));
    println!("[{:10?}]", (1, 2));
    println!("[{:>8?}]", "ab");
    println!("[{:<8?}]", 'c');
    println!("[{:5?}]", P { x: 1, y: 2.0 });
    println!("[{:.1?}]", P { x: 1, y: 2.25 });
    println!("[{:.2?}]", vec![1.0, 2.5]);
    println!("[{:03?}]", vec![1, 2]);
    println!("[{:#x?}]", vec![255, 16]);
    println!("[{:x?}]", Some(171));
    println!("[{:+?}]", 3);
    println!("[{:e}]", 1500i32);
    println!("[{:>+8.1e}]", 0.000123);
    println!("[{}]", i128::MAX);
    println!("[{}]", i128::MIN);
    println!("[{}]", u128::MAX);
    println!("[{:x}]", u128::MAX);
    println!("[{:?}]", 1u128 << 100);
    println!("[{:>40}]", 1i128 << 120);
    println!("[{:^11.3}]", -1.0f32 / 3.0);
    println!("[{:<+7}]", 12);
    println!("[{:^#12b}]", 10);
    println!("[{:0>5}]", 7);
    println!("[{:0<5}]", 7);
    println!("[{:#?}]", ());
    println!("[{:?}]", ((), ((),)));
    println!("[{:?}]", [(); 2]);
    println!("[{:?}]", "tab\there");
    println!("[{}]", "tab\there");
    println!("[{:?}]", "\\");
    println!("[{:?}]", '"');
    println!("[{:?}]", "'");
}
