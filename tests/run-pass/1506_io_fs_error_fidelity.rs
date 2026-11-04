// OpenOptions validation errors carry std's messages; EIO maps to the
// (unstable) InputOutputError kind; Cursor::read_vectored fills all buffers.
use std::fs::{self, File, OpenOptions};
use std::io::{self, Cursor, IoSliceMut, Read};

fn main() {
    let base = std::env::temp_dir().join(format!("rustgo_1506_{}", std::process::id()));
    let _ = fs::remove_dir_all(&base);
    fs::create_dir(&base).unwrap();
    let f = base.join("f");
    fs::write(&f, "data").unwrap();
    let k = |r: io::Result<File>| r.map(|_| ()).map_err(|e| format!("{:?}: {}", e.kind(), e));
    println!("{:?}", k(OpenOptions::new().append(true).truncate(true).open(&f)));
    println!("{:?}", k(OpenOptions::new().open(&f)));
    println!("{:?}", k(OpenOptions::new().create(true).read(true).open(&f)));
    println!("{:?}", k(OpenOptions::new().truncate(true).read(true).open(&f)));
    println!("{:?}", k(OpenOptions::new().create_new(true).append(true).open(base.join("g"))));
    println!("{:?}", k(OpenOptions::new().write(true).create_new(true).open(&f)));
    fs::remove_dir_all(&base).unwrap();

    let e = io::Error::from_raw_os_error(5);
    println!("{:?} {} / {}", e.kind(), e.kind(), e);
    let e2 = io::Error::from(e.kind());
    println!("{} {:?}", e2, e2);

    let mut rd = Cursor::new(b"abcdef".to_vec());
    let mut x1 = [0u8; 2];
    let mut x2 = [0u8; 3];
    let mut x3 = [0u8; 4];
    let n = rd
        .read_vectored(&mut [IoSliceMut::new(&mut x1), IoSliceMut::new(&mut x2), IoSliceMut::new(&mut x3)])
        .unwrap();
    println!("{} {:?} {:?} {:?} {}", n, x1, x2, x3, rd.position());
    let mut y = [0u8; 4];
    println!("{:?} {:?}", rd.read_vectored(&mut [IoSliceMut::new(&mut y)]), y);
}
