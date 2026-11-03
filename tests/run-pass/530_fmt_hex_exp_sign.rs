// integer radix formatting for all widths and signs, exponent formats, Debug hex flags
fn main() {
    println!("{:x} {:X} {:o} {:b}", 255u8, 255u8, 255u8, 255u8);
    println!("{:x} {:x} {:x}", -1i8, -1i16, -1i32);
    println!("{:x}", -1i64);
    println!("{:x}", -1i128);
    println!("{:#x} {:#X} {:#o} {:#b}", 0, 0, 0, 0);
    println!("{:#x} {:#b}", i8::MIN, i8::MIN);
    println!("{:#018x}", u64::MAX / 3);
    println!("{:#06x} {:06x} {:<#6x}| {:^#8b}|", 0xab, 0xab, 0xab, 5);
    println!("{:+x} {:+#x}", 26, 26);
    println!("{:032b}", 0xdeadbeefu32);
    println!("{:o}", u128::MAX);
    println!("{:X}", u128::MAX - 0xFF);
    println!("{:b}", i128::MIN);
    println!("{:e} {:e} {:e} {:e}", 0.0, 1.0, -1.0, 10.0);
    println!("{:e} {:e} {:e}", 123.456, 0.001234, 1e100);
    println!("{:+e} {:+e} {:+E}", 5.5, -5.5, 1e-7);
    println!("{:.2e} {:.0e} {:.5e}", 123.456, 9.5, 1.0);
    println!("{:e} {:e} {:e}", 1u32, 1200u32, 123456789u64);
    println!("{:e} {:E}", -4500i32, 99i8);
    println!("{:.2e} {:.1e}", 1255u32, 95u32);
    println!("{:e} {:e}", f64::MAX, f64::MIN_POSITIVE);
    println!("{:e} {:e}", 5e-324, f32::MAX);
    println!("{:e} {:e} {:e}", f64::NAN, f64::INFINITY, -f64::INFINITY);
    println!("{:12e}| {:<12e}| {:+012e}|", 1.5, 2.5, 3.5);
    println!("{:?} {:?}", 1e15, 1e16);
    println!("{:?} {:?} {:?}", 1e-5, 1.5e-7, 0.0001);
    println!("{} {} {}", 1e15, 1e16, 1e21);
    println!("{} {}", 1e-7, 123456789012345680.0);
    println!("{:?} {:?}", f32::MIN_POSITIVE, 1.0e-40f32);
    println!("{:x?} {:X?} {:#x?}", vec![10, 255], [171u8, 205], Some(4096));
    println!("{:02x?}", [1u8, 0xa, 0xff]);
    let bytes = [0xdeu8, 0xad, 0xbe, 0xef, 0x00, 0x01];
    let hex: String = bytes.iter().map(|b| format!("{:02x}", b)).collect();
    println!("{}", hex);
    let parsed: Vec<u8> = (0..hex.len()).step_by(2).map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap()).collect();
    println!("{:?}", parsed);
    for v in [0i32, 1, -1, 127, -128, i32::MAX, i32::MIN] {
        println!("{:>12} {:>#12x} {:>12o}", v, v, v as u32 >> 20);
    }
    println!("{:#b}", 1u128 << 100);
    println!("{:x}", 0x0123_4567_89ab_cdef_u64.swap_bytes());
    println!("{:?}", format!("{:x}", 3.0f64.to_bits()));
    println!("{:#?}", [0x10u8]);
    println!("{:5x}|{:<5X}|{:^5o}|", 31, 31, 31);
    println!("{}", format!("{:e}", 0.1f32));
    println!("{}", format!("{:e}", 0.3f64));
    println!("{:e}", 1234.5f32);
    println!("{:?} {:?}", 0.1f32 as f64, 1.0f32 / 3.0);
    println!("{} {}", 1.0f64 / 3.0, 2.0f32 / 3.0);
    println!("{} {}", 100000000.0f32, 1.2345678e-8f32);
    println!("{} {}", u64::MAX as f64, u64::MAX as f32);
    #[derive(Debug)]
    struct Reg {
        addr: u32,
        vals: [u8; 3],
        f: f32,
    }
    let r = Reg { addr: 0xbeef, vals: [1, 0x1f, 0xff], f: 0.5 };
    println!("{:x?}", r);
    println!("{:#X?}", r);
    println!("{:?}", r);
    for v in [1u64, 10, 255, 4096, 65535, 1 << 32] {
        println!("{:>12} {:>#14x} {:>#40b}", v, v, v);
    }
    for v in [-1i8, -128, 127] {
        println!("{:#06x} {:#010b} {:o}", v, v, v);
    }
    println!("{:e} {:e} {:e}", 100u8, 0u8, u64::MAX);
    println!("{:.3e} {:10.1e}| {:<10.2E}|", 987654321u32, 123456i64, -0.000999f64);
    println!("{:e} {:e}", 1.0e-7f32, 3.0e38f32);
    println!("{:+.0e} {:+.1e}", 0.0f64, -0.0f64);
    println!("{:X}", i64::MIN);
    println!("{:x}", 0u8);
    println!("{:#x}", u16::MAX as i16);
}
