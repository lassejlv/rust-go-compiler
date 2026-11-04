mod t01 {
    use std::io::{self, BufRead, BufReader, BufWriter, Cursor, Read, Seek, SeekFrom, Write};
    pub fn main() -> io::Result<()> {
        let data = "line one\nline two\n\nlast";
        let r = BufReader::new(Cursor::new(data.as_bytes()));
        for (i, l) in r.lines().enumerate() { println!("{}: {:?}", i, l?); }
        let mut c = Cursor::new(Vec::new());
        write!(c, "hello {}", 5)?;
        c.write_all(b" world")?;
        c.seek(SeekFrom::Start(6))?;
        let mut s = String::new();
        c.read_to_string(&mut s)?;
        println!("{:?} pos={}", s, c.position());
        let inner = c.into_inner();
        println!("{:?}", String::from_utf8(inner).unwrap());
        let mut buf = [0u8; 4];
        let mut rd = Cursor::new(b"abcdefg".to_vec());
        rd.read_exact(&mut buf)?;
        println!("{:?}", std::str::from_utf8(&buf));
        let n = rd.read(&mut buf)?;
        println!("{} {:?}", n, &buf[..n]);
        println!("{:?}", rd.read_exact(&mut buf).map_err(|e| e.kind()));
        let mut br = BufReader::new(Cursor::new("a,b,c"));
        let mut part = Vec::new();
        while br.read_until(b',', &mut part)? > 0 { println!("{:?}", String::from_utf8_lossy(&part)); part.clear(); }
        let split: Vec<Vec<u8>> = BufReader::new(Cursor::new("x;y;z")).split(b';').map(|r| r.unwrap()).collect();
        println!("{:?}", split);
        let mut out = BufWriter::new(Vec::new());
        for i in 0..3 { writeln!(out, "row {}", i)?; }
        out.flush()?;
        let v = out.into_inner().unwrap();
        print!("{}", String::from_utf8(v).unwrap());
        let mut line = String::new();
        let mut r2 = Cursor::new("first\nsecond");
        r2.read_line(&mut line)?;
        r2.read_line(&mut line)?;
        println!("{:?}", line);
        let chained: Vec<u8> = Cursor::new("ab").chain(Cursor::new("cd")).bytes().map(|b| b.unwrap()).collect();
        println!("{:?}", String::from_utf8(chained));
        let mut taken = String::new();
        Cursor::new("abcdef").take(3).read_to_string(&mut taken)?;
        println!("{}", taken);
        let stdout = io::stdout();
        let mut lock = stdout.lock();
        writeln!(lock, "locked write")?;
        let e = io::Error::new(io::ErrorKind::Other, "custom");
        println!("{} {:?}", e, e.kind());
        let e2: io::Error = io::ErrorKind::NotFound.into();
        println!("{}", e2.kind() == io::ErrorKind::NotFound);
        io::copy(&mut Cursor::new("copied\n"), &mut io::stdout())?;
        let mut sink = io::sink();
        writeln!(sink, "nothing")?;
        let mut empty = String::new();
        io::empty().read_to_string(&mut empty)?;
        println!("{:?}", empty);
        Ok(())
    }
}

mod t02 {
    use std::sync::{Arc, Mutex, RwLock, Barrier, mpsc, Condvar};
    use std::sync::atomic::{AtomicUsize, AtomicBool, AtomicI64, Ordering};
    use std::thread;
    pub fn main() {
        let counter = Arc::new(AtomicUsize::new(0));
        let data = Arc::new(Mutex::new(Vec::new()));
        let mut hs = vec![];
        for i in 0..4 {
            let c = Arc::clone(&counter);
            let d = Arc::clone(&data);
            hs.push(thread::spawn(move || {
                for _ in 0..1000 { c.fetch_add(1, Ordering::SeqCst); }
                d.lock().unwrap().push(i);
                i * 10
            }));
        }
        let rets: Vec<i32> = hs.into_iter().map(|h| h.join().unwrap()).collect();
        let mut d = data.lock().unwrap().clone(); d.sort();
        println!("{} {:?} {:?}", counter.load(Ordering::SeqCst), d, rets);
        let (tx, rx) = mpsc::channel();
        for id in 0..3 { let tx = tx.clone(); thread::spawn(move || { for j in 0..3 { tx.send(id * 100 + j).unwrap(); } }); }
        drop(tx);
        let mut got: Vec<i32> = rx.iter().collect(); got.sort();
        println!("{:?}", got);
        let (stx, srx) = mpsc::sync_channel::<String>(1);
        let h = thread::spawn(move || { for w in ["a", "b", "c"] { stx.send(w.to_string()).unwrap(); } });
        let mut words = vec![];
        while let Ok(w) = srx.recv() { words.push(w); }
        h.join().unwrap();
        println!("{:?} {:?}", words, srx.try_recv().is_err());
        let rw = Arc::new(RwLock::new(5));
        { let r1 = rw.read().unwrap(); let r2 = rw.read().unwrap(); println!("{}", *r1 + *r2); }
        *rw.write().unwrap() += 1;
        println!("{:?}", *rw.read().unwrap());
        let mut v = vec![1, 2, 3, 4];
        let total = AtomicI64::new(0);
        thread::scope(|s| {
            for chunk in v.chunks_mut(2) {
                let total = &total;
                s.spawn(move || { for x in chunk.iter_mut() { *x *= 2; total.fetch_add(*x, Ordering::Relaxed); } });
            }
        });
        println!("{:?} {}", v, total.into_inner());
        let barrier = Arc::new(Barrier::new(3));
        let flag = Arc::new(AtomicBool::new(false));
        let hs: Vec<_> = (0..3).map(|_| { let b = barrier.clone(); let f = flag.clone(); thread::spawn(move || { let r = b.wait(); if r.is_leader() { f.store(true, Ordering::Release); } }) }).collect();
        for h in hs { h.join().unwrap(); }
        println!("{}", flag.load(Ordering::Acquire));
        let pair = Arc::new((Mutex::new(false), Condvar::new()));
        let p2 = pair.clone();
        thread::spawn(move || { let (l, c) = &*p2; *l.lock().unwrap() = true; c.notify_one(); });
        let (l, c) = &*pair;
        let mut ready = l.lock().unwrap();
        while !*ready { ready = c.wait(ready).unwrap(); }
        println!("ready {}", *ready);
        let r = thread::spawn(|| { if true { panic!("thread panic") } 1 }).join();
        println!("{}", r.is_err());
        let m = Mutex::new(1);
        let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| { let _g = m.lock().unwrap(); panic!("poison"); }));
        println!("{} {}", res.is_err(), m.is_poisoned());
        println!("{:?}", m.lock().map(|g| *g).unwrap_or_else(|e| *e.into_inner()));
        let au = AtomicUsize::new(5);
        println!("{:?} {:?} {}", au.compare_exchange(5, 7, Ordering::SeqCst, Ordering::SeqCst), au.compare_exchange(5, 9, Ordering::SeqCst, Ordering::SeqCst), au.swap(1, Ordering::SeqCst));
        println!("{:?} {}", au.fetch_update(Ordering::SeqCst, Ordering::SeqCst, |x| Some(x + 10)), au.fetch_max(3, Ordering::SeqCst));
        println!("{}", thread::current().name().unwrap_or("?"));
        let b = thread::Builder::new().name("worker".into()).spawn(|| thread::current().name().map(|s| s.to_string())).unwrap();
        println!("{:?}", b.join().unwrap());
    }
}

mod t04 {
    use std::fs;
    use std::io::Write;
    use std::path::{Path, PathBuf};
    pub fn main() {
        let dir = std::env::temp_dir().join(format!("rg_probe_{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let p = dir.join("a.txt");
        fs::write(&p, "hello\nworld\n").unwrap();
        let mut f = fs::OpenOptions::new().append(true).open(&p).unwrap();
        writeln!(f, "more").unwrap();
        drop(f);
        let s = fs::read_to_string(&p).unwrap();
        println!("{:?}", s);
        println!("{}", fs::metadata(&p).unwrap().len());
        println!("{}", p.exists());
        fs::rename(&p, dir.join("b.txt")).unwrap();
        println!("{} {}", p.exists(), dir.join("b.txt").is_file());
        let mut names: Vec<String> = fs::read_dir(&dir).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().into_owned()).collect();
        names.sort();
        println!("{:?}", names);
        fs::remove_file(dir.join("b.txt")).unwrap();
        fs::remove_dir(&dir).unwrap();
        println!("{:?}", fs::read_to_string(dir.join("nope")).map_err(|e| e.kind()));
        let pb = PathBuf::from("/usr/lib/libfoo.so.1");
        println!("{:?} {:?} {:?} {:?}", pb.file_name(), pb.extension(), pb.file_stem(), pb.parent());
        let mut pb2 = PathBuf::from("a/b");
        pb2.push("c.rs");
        pb2.set_extension("txt");
        println!("{} {:?}", pb2.display(), pb2.components().count());
        println!("{:?}", Path::new("x/y/z").ancestors().collect::<Vec<_>>());
        println!("{}", Path::new("/a").is_absolute());
        println!("{:?}", Path::new("a/b.tar.gz").with_extension("zip"));
        println!("{:?}", Path::new("/a/b").strip_prefix("/a"));
        println!("{:?}", Path::new("/a/b").join("/c"));
    }
}

fn main() {
    println!("== t01");
    t01::main().unwrap();
    println!("== t02");
    t02::main();
    println!("== t04");
    t04::main();
}
