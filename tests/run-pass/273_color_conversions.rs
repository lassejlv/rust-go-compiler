// Color types as in GPUI: Rgba/Hsla with From conversions, hex parsing (#rgb, #rrggbb, #rrggbbaa),
// blending, opacity, lighten/darken, contrast ratio, and Display/Debug formatting.
use std::fmt;
use std::str::FromStr;

#[derive(Clone, Copy, PartialEq, Debug)]
struct Rgba {
    r: f32,
    g: f32,
    b: f32,
    a: f32,
}

#[derive(Clone, Copy, PartialEq, Debug)]
struct Hsla {
    h: f32,
    s: f32,
    l: f32,
    a: f32,
}

fn rgb(hex: u32) -> Rgba {
    Rgba {
        r: ((hex >> 16) & 0xff) as f32 / 255.0,
        g: ((hex >> 8) & 0xff) as f32 / 255.0,
        b: (hex & 0xff) as f32 / 255.0,
        a: 1.0,
    }
}

impl From<Rgba> for Hsla {
    fn from(c: Rgba) -> Hsla {
        let max = c.r.max(c.g).max(c.b);
        let min = c.r.min(c.g).min(c.b);
        let l = (max + min) / 2.0;
        if max == min {
            return Hsla { h: 0.0, s: 0.0, l, a: c.a };
        }
        let d = max - min;
        let s = if l > 0.5 { d / (2.0 - max - min) } else { d / (max + min) };
        let h = if max == c.r {
            (c.g - c.b) / d + if c.g < c.b { 6.0 } else { 0.0 }
        } else if max == c.g {
            (c.b - c.r) / d + 2.0
        } else {
            (c.r - c.g) / d + 4.0
        };
        Hsla { h: h / 6.0, s, l, a: c.a }
    }
}

impl From<Hsla> for Rgba {
    fn from(c: Hsla) -> Rgba {
        if c.s == 0.0 {
            return Rgba { r: c.l, g: c.l, b: c.l, a: c.a };
        }
        fn hue(p: f32, q: f32, mut t: f32) -> f32 {
            if t < 0.0 {
                t += 1.0;
            }
            if t > 1.0 {
                t -= 1.0;
            }
            if t < 1.0 / 6.0 {
                p + (q - p) * 6.0 * t
            } else if t < 0.5 {
                q
            } else if t < 2.0 / 3.0 {
                p + (q - p) * (2.0 / 3.0 - t) * 6.0
            } else {
                p
            }
        }
        let q = if c.l < 0.5 { c.l * (1.0 + c.s) } else { c.l + c.s - c.l * c.s };
        let p = 2.0 * c.l - q;
        Rgba { r: hue(p, q, c.h + 1.0 / 3.0), g: hue(p, q, c.h), b: hue(p, q, c.h - 1.0 / 3.0), a: c.a }
    }
}

#[derive(Debug, PartialEq)]
struct ColorParseError(String);

impl FromStr for Rgba {
    type Err = ColorParseError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let hex = s.strip_prefix('#').ok_or_else(|| ColorParseError(format!("missing '#': {}", s)))?;
        let digit = |i: usize| -> Result<u8, ColorParseError> {
            u8::from_str_radix(&hex[i..i + 1], 16).map_err(|_| ColorParseError(format!("bad hex digit in {}", s)))
        };
        let pair = |i: usize| -> Result<u8, ColorParseError> { Ok(digit(i)? * 16 + digit(i + 1)?) };
        let (r, g, b, a) = match hex.len() {
            3 => (digit(0)? * 17, digit(1)? * 17, digit(2)? * 17, 255),
            6 => (pair(0)?, pair(2)?, pair(4)?, 255),
            8 => (pair(0)?, pair(2)?, pair(4)?, pair(6)?),
            n => return Err(ColorParseError(format!("bad length {}", n))),
        };
        Ok(Rgba { r: r as f32 / 255.0, g: g as f32 / 255.0, b: b as f32 / 255.0, a: a as f32 / 255.0 })
    }
}

impl fmt::Display for Rgba {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let c = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
        if self.a >= 1.0 && !f.alternate() {
            write!(f, "#{:02x}{:02x}{:02x}", c(self.r), c(self.g), c(self.b))
        } else {
            write!(f, "#{:02x}{:02x}{:02x}{:02x}", c(self.r), c(self.g), c(self.b), c(self.a))
        }
    }
}

impl fmt::Display for Hsla {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "hsla({:.0}, {:.0}%, {:.0}%, {:.2})", self.h * 360.0, self.s * 100.0, self.l * 100.0, self.a)
    }
}

impl Hsla {
    fn opacity(self, a: f32) -> Self {
        Hsla { a: self.a * a, ..self }
    }
    fn lighten(self, amt: f32) -> Self {
        Hsla { l: (self.l + amt).min(1.0), ..self }
    }
    fn darken(self, amt: f32) -> Self {
        Hsla { l: (self.l - amt).max(0.0), ..self }
    }
    fn rotate(self, deg: f32) -> Self {
        Hsla { h: (self.h + deg / 360.0).rem_euclid(1.0), ..self }
    }
}

impl Rgba {
    // "over" compositing
    fn blend(self, top: Rgba) -> Rgba {
        let a = top.a + self.a * (1.0 - top.a);
        if a == 0.0 {
            return Rgba { r: 0.0, g: 0.0, b: 0.0, a: 0.0 };
        }
        let mix = |b: f32, t: f32| (t * top.a + b * self.a * (1.0 - top.a)) / a;
        Rgba { r: mix(self.r, top.r), g: mix(self.g, top.g), b: mix(self.b, top.b), a }
    }
    fn luminance(&self) -> f32 {
        let lin = |c: f32| if c <= 0.03928 { c / 12.92 } else { ((c + 0.055) / 1.055).powf(2.4) };
        0.2126 * lin(self.r) + 0.7152 * lin(self.g) + 0.0722 * lin(self.b)
    }
    fn contrast(&self, o: &Rgba) -> f32 {
        let (a, b) = (self.luminance(), o.luminance());
        (a.max(b) + 0.05) / (a.min(b) + 0.05)
    }
}

fn main() {
    let colors = ["#ff0000", "#0f0", "#1e90ff", "#80808080", "#ffffff", "#000", "#c0ffee"];
    for s in colors {
        let c: Rgba = s.parse().unwrap();
        let h: Hsla = c.into();
        let back: Rgba = h.into();
        println!("{:<10} -> {} {:#} -> {} -> {}", s, c, c, h, back);
    }
    for bad in ["ff0000", "#12", "#gg0000", "#1234567"] {
        println!("{:<9} -> {:?}", bad, bad.parse::<Rgba>());
    }
    let base: Hsla = rgb(0x3366cc).into();
    println!("base {}", base);
    println!("lighten {}", Rgba::from(base.lighten(0.2)));
    println!("darken {}", Rgba::from(base.darken(0.3)));
    println!("rotate {}", Rgba::from(base.rotate(180.0)));
    println!("rotate -90 {}", Rgba::from(base.rotate(-90.0)));
    println!("opacity {:#}", Rgba::from(base.opacity(0.5)));
    let bg = rgb(0xffffff);
    let overlay = Rgba { a: 0.25, ..rgb(0x000000) };
    println!("blend {}", bg.blend(overlay));
    println!("blend transparent {:#}", Rgba { r: 0.0, g: 0.0, b: 0.0, a: 0.0 }.blend(Rgba { r: 1.0, g: 0.0, b: 0.0, a: 0.5 }));
    let pairs = [(0x000000, 0xffffff), (0x777777, 0xffffff), (0x1e90ff, 0x000000), (0xff0000, 0x00ff00)];
    for (a, b) in pairs {
        println!("contrast {:06x}/{:06x} = {:.2}", a, b, rgb(a).contrast(&rgb(b)));
    }
    let palette: Vec<String> = (0..6).map(|i| Rgba::from(Hsla { h: i as f32 / 6.0, s: 0.7, l: 0.5, a: 1.0 }).to_string()).collect();
    println!("{}", palette.join(" "));
}
