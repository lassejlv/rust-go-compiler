// A bitflags!-style macro generating a flags struct with consts, operators, Debug and iteration.
use std::fmt;

macro_rules! bitflags {
    (
        $(#[$outer:meta])*
        $vis:vis struct $Name:ident: $T:ty {
            $(
                const $Flag:ident = $value:expr;
            )*
        }
    ) => {
        $(#[$outer])*
        #[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
        $vis struct $Name { bits: $T }

        #[allow(dead_code)]
        impl $Name {
            $(
                pub const $Flag: $Name = $Name { bits: $value };
            )*
            const FLAGS: &'static [(&'static str, $Name)] = &[ $( (stringify!($Flag), $Name::$Flag) ),* ];

            pub const fn empty() -> Self { $Name { bits: 0 } }
            pub const fn all() -> Self { $Name { bits: 0 $( | $value )* } }
            pub const fn bits(&self) -> $T { self.bits }
            pub fn from_bits(bits: $T) -> Option<Self> {
                if bits & !Self::all().bits == 0 { Some($Name { bits }) } else { None }
            }
            pub const fn from_bits_truncate(bits: $T) -> Self { $Name { bits: bits & Self::all().bits } }
            pub const fn is_empty(&self) -> bool { self.bits == 0 }
            pub const fn is_all(&self) -> bool { self.bits == Self::all().bits }
            pub const fn contains(&self, other: Self) -> bool { self.bits & other.bits == other.bits }
            pub const fn intersects(&self, other: Self) -> bool { self.bits & other.bits != 0 }
            pub fn insert(&mut self, other: Self) { self.bits |= other.bits; }
            pub fn remove(&mut self, other: Self) { self.bits &= !other.bits; }
            pub fn toggle(&mut self, other: Self) { self.bits ^= other.bits; }
            pub fn set(&mut self, other: Self, value: bool) {
                if value { self.insert(other) } else { self.remove(other) }
            }
            pub fn iter_names(&self) -> impl Iterator<Item = (&'static str, $Name)> + '_ {
                Self::FLAGS.iter().copied().filter(move |(_, f)| f.bits != 0 && self.contains(*f))
            }
            pub fn from_name(name: &str) -> Option<Self> {
                Self::FLAGS.iter().find(|(n, _)| *n == name).map(|(_, f)| *f)
            }
        }

        impl std::ops::BitOr for $Name {
            type Output = Self;
            fn bitor(self, o: Self) -> Self { $Name { bits: self.bits | o.bits } }
        }
        impl std::ops::BitAnd for $Name {
            type Output = Self;
            fn bitand(self, o: Self) -> Self { $Name { bits: self.bits & o.bits } }
        }
        impl std::ops::BitXor for $Name {
            type Output = Self;
            fn bitxor(self, o: Self) -> Self { $Name { bits: self.bits ^ o.bits } }
        }
        impl std::ops::Sub for $Name {
            type Output = Self;
            fn sub(self, o: Self) -> Self { $Name { bits: self.bits & !o.bits } }
        }
        impl std::ops::Not for $Name {
            type Output = Self;
            fn not(self) -> Self { Self::from_bits_truncate(!self.bits) }
        }
        impl std::ops::BitOrAssign for $Name {
            fn bitor_assign(&mut self, o: Self) { self.bits |= o.bits; }
        }
        impl std::ops::BitAndAssign for $Name {
            fn bitand_assign(&mut self, o: Self) { self.bits &= o.bits; }
        }
        impl FromIterator<$Name> for $Name {
            fn from_iter<I: IntoIterator<Item = $Name>>(it: I) -> Self {
                it.into_iter().fold(Self::empty(), |a, b| a | b)
            }
        }
        impl fmt::Debug for $Name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                if self.is_empty() {
                    return write!(f, "{}({:#x})", stringify!($Name), self.bits);
                }
                let mut first = true;
                write!(f, "{}(", stringify!($Name))?;
                let mut rest = self.bits;
                for (name, flag) in self.iter_names() {
                    if flag.bits & rest == 0 { continue; }
                    if !first { f.write_str(" | ")?; }
                    first = false;
                    f.write_str(name)?;
                    rest &= !flag.bits;
                }
                if rest != 0 {
                    if !first { f.write_str(" | ")?; }
                    write!(f, "{:#x}", rest)?;
                }
                f.write_str(")")
            }
        }
    };
}

bitflags! {
    /// Modifier keys
    pub struct Modifiers: u8 {
        const CONTROL = 1 << 0;
        const ALT = 1 << 1;
        const SHIFT = 1 << 2;
        const PLATFORM = 1 << 3;
        const FUNCTION = 1 << 4;
    }
}

bitflags! {
    struct Perms: u32 {
        const READ = 0b100;
        const WRITE = 0b010;
        const EXEC = 0b001;
        const RW = Self::READ.bits | Self::WRITE.bits;
    }
}

fn parse_keystroke(s: &str) -> (Modifiers, String) {
    let mut mods = Modifiers::empty();
    let mut key = String::new();
    for part in s.split('-') {
        match part {
            "ctrl" => mods |= Modifiers::CONTROL,
            "alt" => mods |= Modifiers::ALT,
            "shift" => mods |= Modifiers::SHIFT,
            "cmd" => mods |= Modifiers::PLATFORM,
            "fn" => mods |= Modifiers::FUNCTION,
            k => key = k.to_string(),
        }
    }
    (mods, key)
}

fn main() {
    let m = Modifiers::CONTROL | Modifiers::SHIFT;
    println!("{:?} bits={:#010b}", m, m.bits());
    println!("contains ctrl={} alt={} intersects(alt|shift)={}", m.contains(Modifiers::CONTROL), m.contains(Modifiers::ALT), m.intersects(Modifiers::ALT | Modifiers::SHIFT));
    println!("{:?}", Modifiers::empty());
    println!("{:?} is_all={}", Modifiers::all(), Modifiers::all().is_all());
    println!("{:?}", !m);
    println!("{:?}", m - Modifiers::CONTROL);
    println!("{:?}", m ^ Modifiers::all());
    let mut x = Modifiers::empty();
    x.insert(Modifiers::ALT);
    x.set(Modifiers::FUNCTION, true);
    x.toggle(Modifiers::ALT | Modifiers::PLATFORM);
    println!("{:?}", x);
    x.set(Modifiers::FUNCTION, false);
    println!("{:?}", x);
    println!("{:?} {:?}", Modifiers::from_bits(0b11), Modifiers::from_bits(0b100000));
    println!("{:?}", Modifiers::from_bits_truncate(0xff));
    println!("{:?}", Modifiers::from_name("SHIFT"));
    let names: Vec<&str> = (Modifiers::ALT | Modifiers::PLATFORM).iter_names().map(|(n, _)| n).collect();
    println!("{:?}", names);
    for ks in ["ctrl-shift-p", "cmd-s", "a", "fn-alt-ctrl-f12"] {
        let (mods, key) = parse_keystroke(ks);
        println!("{:>16} => {:?} + {:?}", ks, mods, key);
    }

    let p = Perms::RW;
    println!("{:?} {:?} {:?}", p, Perms::all(), Perms::READ | Perms::EXEC);
    let collected: Perms = vec![Perms::EXEC, Perms::WRITE].into_iter().collect();
    println!("{:?} {}", collected, collected.contains(Perms::RW));
    let mut sorted = vec![Perms::RW, Perms::EXEC, Perms::empty(), Perms::READ];
    sorted.sort();
    println!("{:?}", sorted);
    let mut q = Perms::all();
    q &= !Perms::WRITE;
    println!("{:?}", q);
}
