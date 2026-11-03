// Deterministic std value types: Ipv4Addr/Ipv6Addr/SocketAddr parsing and
// Display/Debug, Path/PathBuf manipulation (no filesystem access), and
// Duration arithmetic plus its Debug formatting with precision.
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4};
use std::path::{Component, Path, PathBuf};
use std::time::Duration;

fn main() {
    for s in ["127.0.0.1", "10.0.0.255", "192.168.1.1", "256.1.1.1", "1.2.3", "01.2.3.4", "0.0.0.0", "8.8.8.8"] {
        match s.parse::<Ipv4Addr>() {
            Ok(ip) => println!("{:<12} ok  {} loopback={} private={} unspecified={} octets={:?} u32={}", s, ip, ip.is_loopback(), ip.is_private(), ip.is_unspecified(), ip.octets(), u32::from(ip)),
            Err(e) => println!("{:<12} err {}", s, e),
        }
    }
    for s in ["::1", "2001:db8::8a2e:370:7334", "fe80::1", "::", "::ffff:192.168.0.1", "1:2:3:4:5:6:7:8", "1::2::3", "2001:0db8:0000:0000:0000:0000:0000:0001"] {
        match s.parse::<Ipv6Addr>() {
            Ok(ip) => println!("{:<40} {} {:?} loopback={} segs={:x?} v4={:?}", s, ip, ip, ip.is_loopback(), ip.segments(), ip.to_ipv4_mapped()),
            Err(e) => println!("{:<40} err {}", s, e),
        }
    }
    let ips: Vec<IpAddr> = ["1.1.1.1", "::1", "10.1.2.3"].iter().map(|s| s.parse().unwrap()).collect();
    println!("{:?} v4count={}", ips, ips.iter().filter(|i| i.is_ipv4()).count());
    for s in ["127.0.0.1:8080", "[::1]:443", "localhost:80", "1.2.3.4:99999", "[fe80::1]:0"] {
        match s.parse::<SocketAddr>() {
            Ok(a) => println!("{:<16} {} port={} ip={} v6={}", s, a, a.port(), a.ip(), a.is_ipv6()),
            Err(e) => println!("{:<16} err {}", s, e),
        }
    }
    let mut sa = SocketAddrV4::new(Ipv4Addr::new(10, 0, 0, 1), 22);
    sa.set_port(2222);
    println!("{} {:?} {:>20}| {:<20}|", sa, sa, Ipv4Addr::LOCALHOST, Ipv6Addr::UNSPECIFIED);

    let p = Path::new("/usr/local/lib/libfoo.so.1.2");
    println!("path {} parent={:?} file_name={:?} stem={:?} ext={:?}", p.display(), p.parent(), p.file_name(), p.file_stem(), p.extension());
    println!("absolute={} has_root={} starts_with /usr={} ends_with lib/libfoo.so.1.2={}", p.is_absolute(), p.has_root(), p.starts_with("/usr"), p.ends_with("lib/libfoo.so.1.2"));
    let comps: Vec<String> = Path::new("./a/../b//c/./d.txt").components().map(|c| match c {
        Component::RootDir => "/".to_string(),
        Component::CurDir => ".".to_string(),
        Component::ParentDir => "..".to_string(),
        Component::Normal(s) => format!("N({})", s.to_string_lossy()),
        Component::Prefix(_) => "prefix".to_string(),
    }).collect();
    println!("components {:?}", comps);
    let mut pb = PathBuf::from("/home/user");
    pb.push("projects");
    pb.push("demo.tar.gz");
    println!("{} ext={:?}", pb.display(), pb.extension());
    pb.set_extension("zip");
    println!("set_extension -> {}", pb.display());
    pb.set_file_name("other");
    println!("set_file_name -> {}", pb.display());
    let popped = pb.pop();
    println!("pop={} -> {}", popped, pb.display());
    pb.push("/abs/override");
    println!("push absolute replaces: {:?}", pb);
    println!("join {:?} strip_prefix {:?} {:?}", Path::new("rel").join("x/y"), Path::new("/a/b/c").strip_prefix("/a"), Path::new("/a/b").strip_prefix("/z").is_err());
    let anc: Vec<_> = Path::new("/x/y/z").ancestors().map(|a| a.display().to_string()).collect();
    println!("ancestors {:?}", anc);
    println!("with_extension {:?} with_file_name {:?}", Path::new("f.txt").with_extension("md"), Path::new("/d/f.txt").with_file_name("g.rs"));
    println!("hidden stem {:?} ext {:?}; no-ext {:?}", Path::new(".bashrc").file_stem(), Path::new(".bashrc").extension(), Path::new("Makefile").extension());

    let d = Duration::from_millis(1500);
    let durs = [Duration::ZERO, Duration::from_nanos(1), Duration::from_nanos(1500), Duration::from_micros(2500), d, Duration::from_secs(3661), Duration::new(5, 123_456_789), Duration::from_secs_f64(0.25), Duration::MAX];
    for x in durs {
        println!("{:?} | {:.2?} | secs={} millis={} nanos={} f64={}", x, x, x.as_secs(), x.as_millis(), x.subsec_nanos(), x.as_secs_f64());
    }
    println!("add {:?} sub {:?} mul {:?} div {:?}", d + Duration::from_micros(1), d - Duration::from_millis(499), d * 3, d / 4);
    println!("checked_sub {:?} saturating_sub {:?} checked_add MAX {:?}", Duration::from_secs(1).checked_sub(d), Duration::from_secs(1).saturating_sub(d), Duration::MAX.checked_add(Duration::from_nanos(1)));
    println!("mul_f64 {:?} div_f64 {:?} div_duration {}", d.mul_f64(1.5), d.div_f64(3.0), d.div_duration_f64(Duration::from_millis(500)));
    println!("cmp {:?} sum {:?} abs_diff {:?}", d.cmp(&Duration::from_secs(1)), [d, d, Duration::from_nanos(7)].iter().sum::<Duration>(), Duration::from_secs(2).abs_diff(d));
    println!("try_from_secs_f64 {:?} {:?}", Duration::try_from_secs_f64(-1.0).is_err(), Duration::try_from_secs_f32(2.5));
    println!("{:>12?}| {:<10.1?}| {:+?}", Duration::from_millis(12), Duration::from_micros(1234), Duration::from_secs(1));
}
