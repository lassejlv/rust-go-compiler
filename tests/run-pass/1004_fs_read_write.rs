use std::fs::{self, File, OpenOptions};
use std::io::{self, BufRead, BufReader, BufWriter, Read, Seek, SeekFrom, Write};
use std::path::PathBuf;

fn tmp(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("rg1004_{}_{}", std::process::id(), name))
}

fn main() -> io::Result<()> {
    let p = tmp("a.txt");
    fs::write(&p, "line one\nline two\n\nline four\n")?;
    let s = fs::read_to_string(&p)?;
    println!("{:?}", s);
    let f = File::open(&p)?;
    for (i, line) in BufReader::new(f).lines().enumerate() {
        println!("{}: {}", i, line?);
    }
    {
        let mut f = OpenOptions::new().append(true).open(&p)?;
        writeln!(f, "appended {}", 5)?;
        f.write_all(b"raw bytes\n")?;
    }
    println!("len {}", fs::metadata(&p)?.len());
    let bytes = fs::read(&p)?;
    println!("last line {:?}", String::from_utf8_lossy(&bytes).lines().last());
    {
        let f = File::create(&p)?;
        let mut w = BufWriter::new(f);
        for i in 0..5 {
            write!(w, "{},", i * i)?;
        }
        w.flush()?;
    }
    println!("{}", fs::read_to_string(&p)?);
    let mut f = OpenOptions::new().read(true).write(true).open(&p)?;
    f.seek(SeekFrom::Start(2))?;
    f.write_all(b"XX")?;
    f.seek(SeekFrom::Start(0))?;
    let mut buf = String::new();
    f.read_to_string(&mut buf)?;
    println!("after seek-write {}", buf);
    let pos = f.seek(SeekFrom::End(-3))?;
    let mut tail = [0u8; 3];
    f.read_exact(&mut tail)?;
    println!("pos {} tail {:?}", pos, std::str::from_utf8(&tail).unwrap());
    f.set_len(4)?;
    drop(f);
    println!("truncated {:?}", fs::read_to_string(&p)?);
    let q = tmp("b.txt");
    fs::rename(&p, &q)?;
    println!("old exists {} new exists {}", p.exists(), q.exists());
    fs::copy(&q, &p)?;
    println!("copy exists {}", p.exists());
    fs::remove_file(&p)?;
    fs::remove_file(&q)?;
    println!("removed {}", !q.exists());
    match File::open(&q) {
        Err(e) => println!("open missing: {:?}", e.kind()),
        Ok(_) => println!("unexpected"),
    }
    let r = OpenOptions::new().write(true).create_new(true).open(tmp("c.txt"));
    println!("create_new ok {}", r.is_ok());
    let r2 = OpenOptions::new().write(true).create_new(true).open(tmp("c.txt"));
    println!("create_new again {:?}", r2.map(|_| ()).map_err(|e| e.kind()));
    fs::remove_file(tmp("c.txt"))?;
    Ok(())
}
