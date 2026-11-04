// DefaultHasher results are deterministic in std (SipHash-1-3, zero keys):
// the Hash impls of Ordering (repr(i8)), Path (component bytes + chunk bits)
// and Ipv4Addr/Ipv6Addr (one integer) feed the hasher like std. Also a NUL in
// a Command's program is reported with std's message.
use std::collections::hash_map::DefaultHasher;
use std::hash::{BuildHasher, BuildHasherDefault, Hash, Hasher};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::path::{Path, PathBuf};

#[derive(Hash)]
enum E {
    A,
    B(u8),
    C { x: i64, y: String },
}

fn h<T: Hash + ?Sized>(t: &T) -> u64 {
    let mut s = DefaultHasher::new();
    t.hash(&mut s);
    s.finish()
}

fn main() {
    println!("{} {} {}", h(&1i32), h("hello"), h(&(1u8, 'x', "y")));
    println!("{} {} {}", h(&E::A), h(&E::B(3)), h(&E::C { x: -5, y: "q".into() }));
    println!("{} {} {}", h(&std::cmp::Ordering::Less), h(&std::cmp::Ordering::Equal), h(&std::cmp::Ordering::Greater));
    println!("{}", std::mem::size_of::<std::cmp::Ordering>());
    println!("{} {}", std::cmp::Ordering::Less as i8, std::cmp::Ordering::Greater as i32);
    println!("{}", h(Path::new("a/b/c")));
    println!("{}", h(&PathBuf::from("/x//y/./z")) == h(Path::new("/x/y/z")));
    println!("{}", h(Path::new("foo/bar")) != h(Path::new("foobar")));
    println!("{}", h(&PathBuf::from("rel/./dir/")));
    println!("{}", h(&Ipv4Addr::new(1, 2, 3, 4)));
    println!("{}", h(&Ipv6Addr::LOCALHOST));
    println!("{}", h(&IpAddr::V4(Ipv4Addr::LOCALHOST)));
    println!("{}", h(&"127.0.0.1:80".parse::<SocketAddr>().unwrap()));
    let bh = BuildHasherDefault::<DefaultHasher>::default();
    println!("{}", bh.hash_one(Path::new("one/two")));
    let mut s = DefaultHasher::new();
    s.write_u8(1);
    s.write_i128(-6);
    s.write(b"xyz");
    println!("{}", s.finish());

    let e = std::process::Command::new("a\0b").status().unwrap_err();
    println!("{:?}: {}", e.kind(), e);
}
