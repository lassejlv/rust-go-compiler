// A macro that generates a state machine enum plus a transition function
macro_rules! state_machine {
    (
        $name:ident {
            start: $start:ident,
            others: [$($st:ident),*],
            $( $from:ident --$ev:ident--> $to:ident ),* $(,)?
        }
    ) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        #[allow(dead_code)]
        enum $name { $start, $($st,)* }

        mod events {
            #[derive(Debug, Clone, Copy, PartialEq, Eq)]
            #[allow(non_camel_case_types, dead_code)]
            pub enum Event { $($ev),* }
        }

        impl $name {
            fn new() -> Self { $name::$start }
            fn step(self, ev: events::Event) -> Option<Self> {
                $(
                    if self == $name::$from && ev == events::Event::$ev {
                        return Some($name::$to);
                    }
                )*
                None
            }
            fn transitions() -> Vec<String> {
                vec![$(format!("{} -{}-> {}", stringify!($from), stringify!($ev), stringify!($to))),*]
            }
        }
    };
}

state_machine!(Door {
    start: Closed,
    others: [Opened, Locked],
    Closed --open--> Opened,
    Opened --close--> Closed,
    Closed --lock--> Locked,
    Locked --unlock--> Closed,
});

macro_rules! bitflags_lite {
    ($name:ident : $t:ty { $($flag:ident = $val:expr),* }) => {
        #[derive(Clone, Copy, PartialEq, Eq)]
        struct $name($t);
        #[allow(non_upper_case_globals)]
        impl $name {
            $(const $flag: $name = $name($val);)*
            fn empty() -> Self { $name(0) }
            fn contains(self, o: Self) -> bool { self.0 & o.0 == o.0 }
            fn names(self) -> Vec<&'static str> {
                let mut v = vec![];
                $(if self.contains($name::$flag) && $val != 0 { v.push(stringify!($flag)); })*
                v
            }
        }
        impl std::ops::BitOr for $name {
            type Output = Self;
            fn bitor(self, o: Self) -> Self { $name(self.0 | o.0) }
        }
        impl std::fmt::Debug for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                write!(f, "{}({})", stringify!($name), self.names().join(" | "))
            }
        }
    };
}

bitflags_lite!(Perm: u8 { READ = 1, WRITE = 2, EXEC = 4 });

fn main() {
    use events::Event::*;
    let mut d = Door::new();
    println!("{:?}", d);
    for ev in [open, lock, close, lock, open, unlock, open] {
        match d.step(ev) {
            Some(n) => { println!("{:?} --{:?}--> {:?}", d, ev, n); d = n; }
            None => println!("{:?} ignores {:?}", d, ev),
        }
    }
    for t in Door::transitions() { println!("{}", t); }
    let p = Perm::READ | Perm::EXEC;
    println!("{:?} {} {}", p, p.contains(Perm::READ), p.contains(Perm::WRITE));
    println!("{:?}", Perm::empty());
    println!("{:?}", Perm::READ | Perm::WRITE | Perm::EXEC);
}
