use std::iter::FromIterator;

#[derive(Debug)]
struct Countdown {
    lo: u32,
    hi: u32,
}

impl Iterator for Countdown {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        if self.lo >= self.hi {
            None
        } else {
            self.hi -= 1;
            Some(self.hi)
        }
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        let n = (self.hi - self.lo) as usize;
        (n, Some(n))
    }
}

impl DoubleEndedIterator for Countdown {
    fn next_back(&mut self) -> Option<u32> {
        if self.lo >= self.hi {
            None
        } else {
            self.lo += 1;
            Some(self.lo - 1)
        }
    }
}

impl ExactSizeIterator for Countdown {}

#[derive(Debug, Default)]
struct Bag {
    items: Vec<String>,
    total_len: usize,
}

impl Bag {
    fn add(&mut self, s: String) {
        self.total_len += s.len();
        self.items.push(s);
    }
}

impl FromIterator<String> for Bag {
    fn from_iter<I: IntoIterator<Item = String>>(iter: I) -> Self {
        let mut b = Bag::default();
        for s in iter {
            b.add(s);
        }
        b
    }
}

impl<'a> FromIterator<&'a str> for Bag {
    fn from_iter<I: IntoIterator<Item = &'a str>>(iter: I) -> Self {
        iter.into_iter().map(|s| s.to_string()).collect()
    }
}

impl Extend<String> for Bag {
    fn extend<I: IntoIterator<Item = String>>(&mut self, iter: I) {
        for s in iter {
            self.add(s);
        }
    }
}

impl<'a> IntoIterator for &'a Bag {
    type Item = &'a String;
    type IntoIter = std::slice::Iter<'a, String>;
    fn into_iter(self) -> Self::IntoIter {
        self.items.iter()
    }
}

impl IntoIterator for Bag {
    type Item = String;
    type IntoIter = std::vec::IntoIter<String>;
    fn into_iter(self) -> Self::IntoIter {
        self.items.into_iter()
    }
}

fn main() {
    let c = Countdown { lo: 0, hi: 5 };
    println!("len {}", c.len());
    println!("{:?}", c.collect::<Vec<_>>());
    println!("{:?}", Countdown { lo: 0, hi: 5 }.rev().collect::<Vec<_>>());
    let mut c = Countdown { lo: 10, hi: 16 };
    println!("{:?} {:?}", c.next(), c.next_back());
    println!("remaining {} {:?}", c.len(), c);
    let mixed: Vec<u32> = c.by_ref().take(2).collect();
    println!("{:?} then {:?}", mixed, c.collect::<Vec<_>>());
    let v: Vec<(usize, u32)> = Countdown { lo: 1, hi: 4 }.enumerate().rev().collect();
    println!("{:?}", v);
    let z: Vec<(u32, u32)> = Countdown { lo: 0, hi: 3 }.zip(Countdown { lo: 0, hi: 3 }.rev()).collect();
    println!("{:?}", z);
    println!("rposition {:?}", Countdown { lo: 0, hi: 6 }.rposition(|x| x % 2 == 0));
    println!("nth_back {:?}", Countdown { lo: 0, hi: 6 }.nth_back(1));
    println!("rfind {:?}", Countdown { lo: 0, hi: 6 }.rfind(|x| *x > 2));
    println!("rfold {}", Countdown { lo: 0, hi: 4 }.rfold(String::new(), |acc, x| acc + &x.to_string()));

    let mut bag: Bag = vec!["alpha", "beta"].into_iter().collect();
    println!("{:?}", bag);
    bag.extend(vec!["gamma".to_string(), "delta".to_string()]);
    bag.extend("x y".split(' ').map(String::from));
    let mut n = 0;
    for s in &bag {
        n += 1;
        print!("{}:{} ", n, s);
    }
    println!();
    println!("total_len {}", bag.total_len);
    let longest = (&bag).into_iter().max_by_key(|s| s.len()).unwrap();
    println!("longest {}", longest);
    let upper: Bag = bag.into_iter().filter(|s| s.len() > 1).map(|s| s.to_uppercase()).collect();
    println!("{:?}", upper.items);
}
