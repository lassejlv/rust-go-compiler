use std::collections::BTreeMap;
use std::fmt;

#[derive(Debug)]
enum Status { Active, Suspended { days: u32, reason: String }, Gone(Option<u8>) }

#[derive(Debug)]
struct Account { id: u32, owner: (String, char), tags: Vec<&'static str>, status: Status, meta: BTreeMap<String, f32> }

struct Money(i64);
impl fmt::Display for Money {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let sign = if self.0 < 0 { "-" } else if f.sign_plus() { "+" } else { "" };
        let s = format!("{}${}.{:02}", sign, self.0.abs() / 100, self.0.abs() % 100);
        f.pad(&s)
    }
}

struct Hexy(u32);
impl fmt::LowerHex for Hexy {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let v = self.0;
        fmt::LowerHex::fmt(&v, f)
    }
}
impl fmt::Binary for Hexy {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "b:")?;
        fmt::Binary::fmt(&self.0, f)
    }
}

fn main() {
    let pi = 3.14159265358979_f64;
    println!("[{:>8.3}] [{:<8.3}] [{:^8.3}] [{:08.3}] [{:+.2}] [{:+08.1}]", pi, pi, pi, pi, pi, -pi);
    println!("[{:e}] [{:E}] [{:.2e}] [{:12.3e}] [{:e}] [{:e}]", 1234.5f64, 0.00012f64, 6.02214076e23f64, -1.5e-7f64, 0.0f64, 1.0f32);
    println!("[{:+}] [{:+}] [{:+}] [{:+.0}]", 5, -5, 0, 2.5f64);
    println!("[{:#x}] [{:#X}] [{:#o}] [{:#b}] [{:x}] [{:X}]", 255, 3054, 8, 5u8, -1i8, 0xdead_beefu32);
    println!("[{:08b}] [{:#010b}] [{:#06x}] [{:>#8x}] [{:<#8x}|] [{:^#9o}]", 37u8, 37u8, 255, 255, 255, 64);
    let w = 10; let p = 2;
    println!("[{:w$.p$}] [{:>w$}]", 2.0f64 / 3.0, "right");
    println!("[{:1$}] [{:.*}]", 7, 6, 1.0f64 / 7.0);
    println!("[{:>1$.2$}] [{0:>2$.1$}]", 9.87654, 12, 3);
    println!("[{:>width$}] [{:^width$.prec$}]", "x", 123.456789, width = 9, prec = 1);
    println!("[{:*^15}] [{:-<10}] [{:#>6}] [{:0>5}] [{:0<5}]", "mid", "left", 42, 7, 7);
    println!("[{:5}] [{:<5}] [{:^5}] [{:>5}]", true, 'c', "ab", 1.5);
    println!("[{:?}] [{:?}] [{:?}] [{:?}]", "quote\"tab\t", 'x', '\n', "unicodé ☃");
    println!("[{:10?}] [{:<10?}] [{:>12?}]", Some(1), "s", (1, 2));
    println!("[{:.3?}] [{:8.2?}] [{:?}] [{:?}]", 1.0f64, vec![1.234, 5.678], 1e21f64, 1e-7f64);
    println!("[{:x?}] [{:X?}] [{:#x?}]", vec![10, 255], [171u8, 205], Some(255));
    println!("[{}] [{:>10}] [{:<10}] [{:+}] [{:^12}]", Money(12345), Money(-99), Money(5), Money(250), Money(-100001));
    println!("[{:x}] [{:#x}] [{:010x}] [{:b}] [{:#014b}]", Hexy(48879), Hexy(48879), Hexy(48879), Hexy(10), Hexy(10));
    println!("{{literal}} {{{}}} }}{{", 5);
    let name = "inline"; let val = 3.5;
    println!("{name}={val:>8.2} {val:e} {name:?} {0} {0:?}", name);
    let mut meta = BTreeMap::new();
    meta.insert("score".to_string(), 9.5f32);
    meta.insert("age".to_string(), 31.0);
    let acc = Account { id: 7, owner: ("Ann".into(), 'A'), tags: vec!["vip", "beta"], status: Status::Suspended { days: 3, reason: "audit".into() }, meta };
    println!("{:?}", acc);
    println!("{:#?}", acc);
    let others = vec![Status::Active, Status::Gone(None), Status::Gone(Some(4))];
    println!("{:#?}", others);
    let empty: Vec<i32> = vec![];
    let empty_map: BTreeMap<i32, i32> = BTreeMap::new();
    println!("{:#?} {:#?} {:#?}", empty, empty_map, ());
    println!("{:?} {:?}", i128::MIN, u128::MAX);
    println!("{:>40}", u128::MAX);
    println!("[{:e}] [{:e}] [{:e}]", 100u32, 123456789u64, -1500i32);
    println!("[{:5}|{:<5}|{:^5}|{:>5}]", "toolong", "toolong", "toolong", "toolong");
    println!("[{:.2}] [{:5.1}] [{:.0}]", "truncate", "abc", "gone");
}
