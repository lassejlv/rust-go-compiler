use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct Date { y: i32, m: u8, d: u8 }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Weekday { Mon, Tue, Wed, Thu, Fri, Sat, Sun }

const MONTHS: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];

const fn is_leap(y: i32) -> bool { (y % 4 == 0 && y % 100 != 0) || y % 400 == 0 }
const fn days_in_month(y: i32, m: u8) -> u8 {
    match m { 1 | 3 | 5 | 7 | 8 | 10 | 12 => 31, 4 | 6 | 9 | 11 => 30, 2 => if is_leap(y) { 29 } else { 28 }, _ => 0 }
}
const DAYS_2024: u32 = { let mut m = 1; let mut t = 0u32; while m <= 12 { t += days_in_month(2024, m) as u32; m += 1; } t };

impl Date {
    fn new(y: i32, m: u8, d: u8) -> Option<Date> { if m >= 1 && m <= 12 && d >= 1 && d <= days_in_month(y, m) { Some(Date { y, m, d }) } else { None } }
    fn to_days(self) -> i64 {
        // days since 1970-01-01 (civil from days algorithm, inverse)
        let y = self.y as i64 - if self.m <= 2 { 1 } else { 0 };
        let era = if y >= 0 { y } else { y - 399 } / 400;
        let yoe = y - era * 400;
        let mp = (self.m as i64 + 9) % 12;
        let doy = (153 * mp + 2) / 5 + self.d as i64 - 1;
        let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
        era * 146097 + doe - 719468
    }
    fn from_days(z: i64) -> Date {
        let z = z + 719468;
        let era = if z >= 0 { z } else { z - 146096 } / 146097;
        let doe = z - era * 146097;
        let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
        let y = yoe + era * 400;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let d = (doy - (153 * mp + 2) / 5 + 1) as u8;
        let m = if mp < 10 { mp + 3 } else { mp - 9 } as u8;
        Date { y: (y + if m <= 2 { 1 } else { 0 }) as i32, m, d }
    }
    fn weekday(self) -> Weekday {
        use Weekday::*;
        [Thu, Fri, Sat, Sun, Mon, Tue, Wed][self.to_days().rem_euclid(7) as usize]
    }
    fn add_days(self, n: i64) -> Date { Date::from_days(self.to_days() + n) }
    fn add_months(self, n: i32) -> Date {
        let total = self.y * 12 + (self.m as i32 - 1) + n;
        let (y, m) = (total.div_euclid(12), (total.rem_euclid(12) + 1) as u8);
        Date { y, m, d: self.d.min(days_in_month(y, m)) }
    }
    fn day_of_year(self) -> u32 { (1..self.m).map(|m| days_in_month(self.y, m) as u32).sum::<u32>() + self.d as u32 }
}

impl fmt::Display for Date {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if f.alternate() { write!(f, "{} {} {}", self.d, MONTHS[self.m as usize - 1], self.y) } else { write!(f, "{:04}-{:02}-{:02}", self.y, self.m, self.d) }
    }
}

impl std::str::FromStr for Date {
    type Err = String;
    fn from_str(s: &str) -> Result<Date, String> {
        let parts: Vec<&str> = s.split('-').collect();
        let [y, m, d] = parts[..] else { return Err(format!("expected Y-M-D, got {:?}", s)) };
        let p = |x: &str| x.parse::<i64>().map_err(|e| format!("{}: {}", x, e));
        Date::new(p(y)? as i32, p(m)? as u8, p(d)? as u8).ok_or_else(|| format!("invalid date {}", s))
    }
}

fn calendar(y: i32, m: u8) -> String {
    let first = Date { y, m, d: 1 };
    let offset = first.weekday() as usize;
    let mut s = format!("{:^20}\nMo Tu We Th Fr Sa Su\n", format!("{} {}", MONTHS[m as usize - 1], y));
    s += &"   ".repeat(offset);
    for d in 1..=days_in_month(y, m) {
        s += &format!("{:>2}", d);
        s.push(if (offset + d as usize) % 7 == 0 { '\n' } else { ' ' });
    }
    s.trim_end().to_string()
}

fn main() {
    let dates = ["1970-01-01", "2000-02-29", "2024-03-15", "1969-07-20", "2100-02-29", "2023-13-01", "abc", "1900-03-01", "0001-01-01", "2038-01-19"];
    for s in dates {
        match s.parse::<Date>() {
            Ok(d) => println!("{} {:#} days={} {:?} doy={} +100={} -1={} +1m={}", d, d, d.to_days(), d.weekday(), d.day_of_year(), d.add_days(100), d.add_days(-1), d.add_months(1)),
            Err(e) => println!("error: {}", e),
        }
    }
    let a = Date::new(2024, 1, 31).unwrap();
    for k in [1, 2, 13, -1, -11, -12, 25] { print!("{} ", a.add_months(k)); }
    println!();
    println!("DAYS_2024={} leap: {:?}", DAYS_2024, [1900, 2000, 2023, 2024, 2400].map(is_leap));
    let mut v: Vec<Date> = ["2024-05-01", "2023-12-31", "2024-01-15", "2024-05-01", "1999-09-09"].iter().map(|s| s.parse().unwrap()).collect();
    v.sort();
    v.dedup();
    println!("{}", v.iter().map(|d| d.to_string()).collect::<Vec<_>>().join(", "));
    let span = v.last().unwrap().to_days() - v[0].to_days();
    println!("span {} days ~ {:.2} years", span, span as f64 / 365.2425);
    println!("{}", calendar(2024, 2));
    println!("{}", calendar(2026, 11));
    let fridays13 = (0..120).map(|i| Date { y: 2020, m: 1, d: 13 }.add_months(i)).filter(|d| d.weekday() == Weekday::Fri).count();
    println!("friday 13ths 2020-2029: {}", fridays13);
    let roundtrip_ok = (-800000..800000).step_by(997).all(|n| Date::from_days(n).to_days() == n);
    println!("roundtrip ok {}", roundtrip_ok);
}
