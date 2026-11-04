// File::try_lock / try_lock_shared and fs::TryLockError.
use std::fs::{File, TryLockError};

fn main() {
    let p = std::env::temp_dir().join(format!("rustgo_1501_lock_{}", std::process::id()));
    let f = File::create(&p).unwrap();
    let g = File::open(&p).unwrap();
    f.lock().unwrap();
    let e = g.try_lock().unwrap_err();
    println!("{:?} / {}", e, e);
    println!("{}", matches!(e, TryLockError::WouldBlock));
    let ioe: std::io::Error = e.into();
    println!("{:?} {}", ioe.kind(), ioe);
    println!("{:?}", g.try_lock_shared().is_err());
    f.unlock().unwrap();
    println!("{:?}", g.try_lock_shared());
    println!("{:?}", f.try_lock_shared());
    let e = f.try_lock().unwrap_err();
    let be: Box<dyn std::error::Error> = Box::new(e);
    println!("{}", be);
    g.unlock().unwrap();
    f.unlock().unwrap();
    println!("{:?}", f.try_lock());
    match g.try_lock() {
        Ok(()) => println!("unexpected"),
        Err(TryLockError::WouldBlock) => println!("held by f"),
        Err(TryLockError::Error(e)) => println!("error {}", e),
    }
    drop(f);
    println!("{:?}", g.try_lock());
    std::fs::remove_file(&p).unwrap();
}
