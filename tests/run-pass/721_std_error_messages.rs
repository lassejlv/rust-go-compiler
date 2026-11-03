// Display/Debug output of std error types: ParseIntError (all kinds),
// ParseFloatError, ParseBoolError, ParseCharError, TryFromIntError,
// Utf8Error / FromUtf8Error, char::from_u32 failures, and Box<dyn Error>.
use std::convert::TryFrom;
use std::error::Error;
use std::num::IntErrorKind;

fn show<T: std::fmt::Debug, E: std::fmt::Display + std::fmt::Debug>(label: &str, r: Result<T, E>) {
    match r {
        Ok(v) => println!("{:<28} ok   {:?}", label, v),
        Err(e) => println!("{:<28} err  {} | {:?}", label, e, e),
    }
}

fn kind_name(k: &IntErrorKind) -> &'static str {
    match k {
        IntErrorKind::Empty => "Empty",
        IntErrorKind::InvalidDigit => "InvalidDigit",
        IntErrorKind::PosOverflow => "PosOverflow",
        IntErrorKind::NegOverflow => "NegOverflow",
        IntErrorKind::Zero => "Zero",
        _ => "other",
    }
}

fn sum_strs(xs: &[&str]) -> Result<i64, Box<dyn Error>> {
    let mut t = 0i64;
    for x in xs {
        let v: i32 = x.trim().parse()?;
        let small = u8::try_from(v)?;
        t += small as i64;
    }
    if t > 300 {
        return Err(format!("total {} too large", t).into());
    }
    Ok(t)
}

fn main() {
    show("u8 '256'", "256".parse::<u8>());
    show("u8 '-1'", "-1".parse::<u8>());
    show("i8 '-129'", "-129".parse::<i8>());
    show("i32 ''", "".parse::<i32>());
    show("i32 ' 5'", " 5".parse::<i32>());
    show("i32 '+5'", "+5".parse::<i32>());
    show("i32 '-'", "-".parse::<i32>());
    show("i32 '+'", "+".parse::<i32>());
    show("u32 '-0'", "-0".parse::<u32>());
    show("i64 '0x10'", "0x10".parse::<i64>());
    show("u128 max", "340282366920938463463374607431768211455".parse::<u128>());
    show("u128 max+1", "340282366920938463463374607431768211456".parse::<u128>());
    show("radix 16 'ff'", i32::from_str_radix("ff", 16));
    show("radix 2 '102'", u8::from_str_radix("102", 2));
    show("radix 36 'zz'", u16::from_str_radix("zz", 36));
    show("radix 16 '-80' i8", i8::from_str_radix("-80", 16));
    show("NonZero '0'", "0".parse::<std::num::NonZeroU32>());
    for s in ["", "x", "999", "-999", "0"] {
        let e = s.parse::<std::num::NonZeroI8>().unwrap_err();
        print!("{}={} ", if s.is_empty() { "<empty>" } else { s }, kind_name(e.kind()));
    }
    println!();

    show("f64 'abc'", "abc".parse::<f64>());
    show("f64 ''", "".parse::<f64>());
    show("f64 '1e400'", "1e400".parse::<f64>());
    show("f64 '-inf'", "-inf".parse::<f64>());
    show("f64 'NaN'", "NaN".parse::<f64>().map(|x| x.is_nan()));
    show("f64 '.5'", ".5".parse::<f64>());
    show("f64 '5.'", "5.".parse::<f64>());
    show("f64 '1_000'", "1_000".parse::<f64>());
    show("f32 '3.4e39'", "3.4e39".parse::<f32>());
    show("bool 'yes'", "yes".parse::<bool>());
    show("bool 'true'", "true".parse::<bool>());
    show("char 'ab'", "ab".parse::<char>());
    show("char ''", "".parse::<char>());
    show("char 'é'", "é".parse::<char>());

    show("u8::try_from(300)", u8::try_from(300i32));
    show("i8::try_from(-129i64)", i8::try_from(-129i64));
    show("u32::try_from(-1)", u32::try_from(-1i32));
    show("usize::try_from(5u64)", usize::try_from(5u64));
    show("char::try_from(0xD800)", char::try_from(0xD800u32));
    show("char::try_from(65)", char::try_from(65u32));
    show("u8 from char 'é'", u8::try_from('é'));
    show("<[u8;2]>::try_from", <[u8; 2]>::try_from(&[1u8, 2, 3][..]));

    let bad = vec![b'h', b'i', 0xff, b'!', 0xe2, 0x82];
    show("from_utf8 bad", std::str::from_utf8(&bad));
    let e = std::str::from_utf8(&bad).unwrap_err();
    println!("valid_up_to={} error_len={:?}", e.valid_up_to(), e.error_len());
    let trunc = [b'a', 0xe2, 0x82];
    let e2 = std::str::from_utf8(&trunc).unwrap_err();
    println!("truncated: {} valid_up_to={} error_len={:?}", e2, e2.valid_up_to(), e2.error_len());
    match String::from_utf8(bad.clone()) {
        Ok(_) => {}
        Err(e) => println!("FromUtf8Error: {} bytes={:?}", e, e.as_bytes()),
    }
    println!("lossy: {}", String::from_utf8_lossy(&bad));
    show("from_utf16 bad", String::from_utf16(&[0x48, 0xD800, 0x69]));

    show("sum ok", sum_strs(&["1", " 2 ", "3"]));
    show("sum parse", sum_strs(&["1", "two"]));
    show("sum range", sum_strs(&["1", "-4"]));
    show("sum big", sum_strs(&["200", "200"]));
    let err = sum_strs(&["x"]).unwrap_err();
    println!("is ParseIntError: {}", err.downcast_ref::<std::num::ParseIntError>().is_some());
    let err = sum_strs(&["999"]).unwrap_err();
    println!("is TryFromIntError: {} source none: {}", err.is::<std::num::TryFromIntError>(), err.source().is_none());
}
