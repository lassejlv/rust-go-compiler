// io::Error kinds/messages from the OS (raw_os_error, Display), newer ErrorKind variants,
// FileExt positional reads/writes, try_clone sharing the cursor, File::options, sync_all.
use std::fs::{self, File};
use std::io::{ErrorKind, Read, Seek, SeekFrom, Write};
use std::os::unix::fs::FileExt;

fn show<T: std::fmt::Debug>(what: &str, r: std::io::Result<T>) {
    match r {
        Ok(v) => println!("{}: ok {:?}", what, v),
        Err(e) => println!("{}: {:?} os={:?} msg={}", what, e.kind(), e.raw_os_error(), e),
    }
}

fn main() {
    let dir = std::env::temp_dir().join(format!("rg_1316_{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir(&dir).unwrap();
    show("create existing dir", fs::create_dir(&dir));
    show("open missing", File::open(dir.join("nope")).map(|_| ()));
    show("read_dir missing", fs::read_dir(dir.join("nope/deeper")).map(|_| ()));
    fs::write(dir.join("f.txt"), b"0123456789").unwrap();
    fs::create_dir(dir.join("sub")).unwrap();
    fs::write(dir.join("sub/x"), b"x").unwrap();
    show("remove non-empty dir", fs::remove_dir(dir.join("sub")));
    show("read dir as file", fs::read_to_string(dir.join("sub")));
    show("file as dir", fs::read_dir(dir.join("f.txt")).map(|_| ()));
    show("remove_file on dir", fs::remove_file(dir.join("sub")));
    show("utf8 error", {
        fs::write(dir.join("bin"), [0xff, 0xfe]).unwrap();
        fs::read_to_string(dir.join("bin"))
    });

    let f = File::options().read(true).write(true).open(dir.join("f.txt")).unwrap();
    let mut buf = [0u8; 4];
    f.read_at(&mut buf, 3).unwrap();
    println!("read_at 3: {:?}", std::str::from_utf8(&buf).unwrap());
    f.write_at(b"AB", 8).unwrap();
    f.write_all_at(b"ZZ", 12).unwrap();
    let mut exact = [0u8; 3];
    f.read_exact_at(&mut exact, 7).unwrap();
    println!("read_exact_at 7: {:?}", std::str::from_utf8(&exact).unwrap());
    let mut a = f.try_clone().unwrap();
    let mut b = f;
    a.seek(SeekFrom::Start(2)).unwrap();
    let mut one = [0u8; 2];
    b.read_exact(&mut one).unwrap();
    println!("clone shares cursor: {:?} pos {}", std::str::from_utf8(&one).unwrap(), a.stream_position().unwrap());
    a.seek(SeekFrom::End(0)).unwrap();
    a.write_all(b"!").unwrap();
    a.sync_all().unwrap();
    b.sync_data().unwrap();
    drop(a);
    drop(b);
    let content = fs::read(dir.join("f.txt")).unwrap();
    println!("content {:?}", String::from_utf8_lossy(&content));
    println!("len {}", fs::metadata(dir.join("f.txt")).unwrap().len());
    let mut r = File::open(dir.join("f.txt")).unwrap();
    let mut s = String::new();
    r.read_to_string(&mut s).unwrap();
    show("write to read-only handle", r.write(b"x"));
    show("read_exact past end", { let mut big = [0u8; 100]; r.read_exact(&mut big) });
    let e = std::io::Error::from_raw_os_error(13);
    println!("{:?} {}", e.kind(), e);
    let custom = std::io::Error::new(ErrorKind::TimedOut, "took too long");
    println!("{} {:?} inner={:?}", custom, custom.kind(), custom.get_ref().map(|i| i.to_string()));
    fs::remove_dir_all(&dir).unwrap();
    show("remove again", fs::remove_dir_all(&dir));
}
