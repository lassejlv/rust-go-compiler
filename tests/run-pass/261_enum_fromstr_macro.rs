// Macros defining enums together with FromStr, Display, iteration over all variants,
// discriminant conversions (TryFrom<u8>), and serde-like rename attributes.
use std::fmt;
use std::str::FromStr;

#[derive(Debug, PartialEq)]
struct ParseEnumError {
    ty: &'static str,
    input: String,
}

impl fmt::Display for ParseEnumError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "invalid {} {:?}", self.ty, self.input)
    }
}

macro_rules! string_enum {
    (
        $(#[$m:meta])*
        enum $name:ident {
            $( $variant:ident = $s:literal $(| $alias:literal)* ),+ $(,)?
        }
    ) => {
        $(#[$m])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
        enum $name { $($variant),+ }

        impl $name {
            const ALL: &'static [$name] = &[$($name::$variant),+];
            fn as_str(&self) -> &'static str {
                match self { $($name::$variant => $s),+ }
            }
            fn iter() -> impl Iterator<Item = $name> { Self::ALL.iter().copied() }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.pad(self.as_str())
            }
        }

        impl FromStr for $name {
            type Err = ParseEnumError;
            fn from_str(s: &str) -> Result<Self, Self::Err> {
                let lower = s.to_ascii_lowercase();
                match lower.as_str() {
                    $( $s $(| $alias)* => Ok($name::$variant), )+
                    _ => Err(ParseEnumError { ty: stringify!($name), input: s.to_string() }),
                }
            }
        }
    };
}

string_enum! {
    /// Programming languages
    enum Language {
        Rust = "rust" | "rs",
        TypeScript = "typescript" | "ts" | "tsx",
        Python = "python" | "py",
        Go = "go" | "golang",
    }
}

string_enum! {
    enum Severity { Hint = "hint", Info = "info", Warning = "warning" | "warn", Error = "error" | "err" }
}

macro_rules! int_enum {
    ($name:ident : $repr:ty { $($v:ident = $n:expr),+ $(,)? }) => {
        #[derive(Debug, Clone, Copy, PartialEq)]
        #[repr($repr)]
        enum $name { $($v = $n),+ }

        impl TryFrom<$repr> for $name {
            type Error = $repr;
            fn try_from(x: $repr) -> Result<Self, $repr> {
                $( if x == $n { return Ok($name::$v); } )+
                Err(x)
            }
        }
        impl From<$name> for $repr {
            fn from(v: $name) -> $repr { v as $repr }
        }
    };
}

int_enum!(Opcode: u8 { Nop = 0x00, Push = 0x10, Pop = 0x11, Add = 0x20, Jmp = 0x30, Halt = 0xFF });

fn disassemble(bytes: &[u8]) -> Vec<String> {
    let mut out = vec![];
    let mut i = 0;
    while i < bytes.len() {
        match Opcode::try_from(bytes[i]) {
            Ok(Opcode::Push) | Ok(Opcode::Jmp) => {
                let op = Opcode::try_from(bytes[i]).unwrap();
                let arg = bytes.get(i + 1).copied().unwrap_or(0);
                out.push(format!("{:?} {}", op, arg));
                i += 2;
            }
            Ok(op) => {
                out.push(format!("{:?}", op));
                i += 1;
            }
            Err(b) => {
                out.push(format!("<bad {:#04x}>", b));
                i += 1;
            }
        }
    }
    out
}

fn main() {
    for s in ["rust", "RS", "tsx", "Golang", "java", ""] {
        match s.parse::<Language>() {
            Ok(l) => println!("{:<8} -> {:?} ({})", format!("{:?}", s), l, l),
            Err(e) => println!("{:<8} -> error: {}", format!("{:?}", s), e),
        }
    }
    let all: Vec<String> = Language::iter().map(|l| format!("[{:^12}]", l)).collect();
    println!("{}", all.join(""));
    println!("{} languages, max = {:?}", Language::ALL.len(), Language::iter().max());

    let mut sevs: Vec<Severity> = "err warn hint info error warning".split(' ').map(|s| s.parse().unwrap()).collect();
    sevs.sort();
    sevs.dedup();
    println!("{:?}", sevs);
    println!("{}", sevs.iter().map(|s| s.to_string()).collect::<Vec<_>>().join(" < "));
    println!("{:?}", "fatal".parse::<Severity>());

    let prog = [0x10, 5, 0x10, 7, 0x20, 0x11, 0x42, 0x30, 0, 0xFF];
    for line in disassemble(&prog) {
        println!("  {}", line);
    }
    let b: u8 = Opcode::Halt.into();
    println!("{} {:?} {:?}", b, Opcode::try_from(0x20u8), Opcode::try_from(0x21u8));
    println!("{}", Opcode::Add as u8 + Opcode::Push as u8);
}
