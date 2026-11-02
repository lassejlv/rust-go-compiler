fn main() {
    let n = 255;
    println!("{:x} {:X} {:o} {:b}", n, n, n, n);
    println!("{:#x} {:#X} {:#o} {:#b}", n, n, n, n);
    println!("{:08b}", 5u8);
    println!("{:#010b}", 5u8);
    println!("{:#06x}", 255);
    println!("{:x}", -1i32);
    println!("{:X}", -2i8);
    println!("{:b}", -1i8);
    println!("{:o}", 8u64);
    println!("{:x}", u64::MAX);
    println!("{:x}", 0);
    println!("{:+}", 5);
    println!("{:+}", -5);
    println!("{:+}", 0);
    println!("{:+.2}", 1.5);
    println!("{:e}", 1234.5);
    println!("{:E}", 0.00012);
    println!("{:e}", 1.0);
    println!("{:e}", 0.0);
    println!("{:e}", 123456789u32);
    println!("{:.2e}", 123456.0);
    println!("{:>12.3e}", 6.02214076e23);
    println!("{:10x}|", 3054);
    println!("{:<#10x}|", 3054);
    println!("{:^+9}|", 42);
    println!("{:.0}", 2.5);
    println!("{:.0}", 3.5);
    println!("{:.1}", 0.25);
    println!("{:.1}", 0.35);
    println!("{:.2}", 1.005);
    println!("{:.3}", 2.0f32 / 3.0);
    println!("{:.0}", -0.4);
    println!("{:.5}", 1e-7);
    println!("{:.20}", 0.1);
    println!("{}", 1u64 << 40);
    println!("{:>20}", i64::MIN);
    for i in [1u32, 10, 100, 1000] {
        println!("{:>6} {:>6x} {:>12b}", i, i, i);
    }
    let rgb = (18u8, 52u8, 86u8);
    println!("#{:02x}{:02x}{:02X}", rgb.0, rgb.1, rgb.2);
}
