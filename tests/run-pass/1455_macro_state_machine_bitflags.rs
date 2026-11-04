// a realistic macro-generated state machine + bitflags-like macro
macro_rules! state_machine {
    (
        $name:ident ($ev:ident) {
            initial: $init:ident,
            states: [$($st:ident),*],
            events: [$($evn:ident),*],
            $( $from:ident + $event:ident => $to:ident ),* $(,)?
        }
    ) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        enum $name { $init, $($st,)* }
        #[allow(dead_code)]
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        enum $ev { $($evn),* }
        impl $name {
            fn new() -> Self { $name::$init }
            fn step(self, e: $ev) -> Option<Self> {
                #[allow(unreachable_patterns)]
                match (self, e) {
                    $( ($name::$from, $ev::$event) => Some($name::$to), )*
                    _ => None,
                }
            }
        }
    };
}

state_machine! {
    Door (DoorEv) {
        initial: Closed,
        states: [Opened, Closed2, Locked],
        events: [Open, Close, Lock, Unlock],
        Closed + Open => Opened,
        Opened + Close => Closed2,
        Closed2 + Lock => Locked,
        Locked + Unlock => Closed,
        Closed2 + Open => Opened,
    }
}

macro_rules! bitflags_lite {
    ($vis:vis struct $name:ident: $t:ty { $( const $flag:ident = $val:expr; )* }) => {
        #[derive(Clone, Copy, PartialEq, Eq, Hash, Default)]
        $vis struct $name { bits: $t }
        #[allow(dead_code)]
        impl $name {
            $( $vis const $flag: $name = $name { bits: $val }; )*
            $vis const fn empty() -> Self { $name { bits: 0 } }
            $vis const fn all() -> Self { $name { bits: 0 $(| $val)* } }
            $vis fn contains(&self, o: Self) -> bool { self.bits & o.bits == o.bits }
            $vis fn bits(&self) -> $t { self.bits }
        }
        impl std::ops::BitOr for $name {
            type Output = Self;
            fn bitor(self, o: Self) -> Self { $name { bits: self.bits | o.bits } }
        }
        impl std::fmt::Debug for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                let mut first = true;
                write!(f, "{}(", stringify!($name))?;
                $(
                    if self.contains($name::$flag) && $name::$flag.bits != 0 {
                        if !first { write!(f, " | ")?; }
                        first = false;
                        write!(f, "{}", stringify!($flag))?;
                    }
                )*
                if first { write!(f, "empty")?; }
                write!(f, ")")
            }
        }
    };
}

bitflags_lite! {
    pub struct Perm: u8 {
        const READ = 1 << 0;
        const WRITE = 1 << 1;
        const EXEC = 1 << 2;
    }
}

fn main() {
    let mut d = Door::new();
    for ev in [DoorEv::Open, DoorEv::Close, DoorEv::Open, DoorEv::Lock, DoorEv::Unlock] {
        match d.step(ev) {
            Some(n) => { println!("{:?} --{:?}--> {:?}", d, ev, n); d = n; }
            None => println!("{:?} ignores {:?}", d, ev),
        }
    }
    let p = Perm::READ | Perm::EXEC;
    println!("{:?} {:?} {:?} {}", p, Perm::empty(), Perm::all(), Perm::all().bits());
    println!("{} {}", p.contains(Perm::READ), p.contains(Perm::WRITE));
    println!("{:?}", Perm::default());
}
