// Display of these std errors goes through Formatter::pad like std (width and
// alignment apply); env::Args/ArgsOs Debug use std's field name.
use std::sync::mpsc;

fn main() {
    let ape = "x".parse::<std::net::IpAddr>().unwrap_err();
    let ape6 = "x".parse::<std::net::SocketAddrV6>().unwrap_err();
    let pse = std::path::Path::new("/a").strip_prefix("/b").unwrap_err();
    println!("[{:>30}]", ape);
    println!("[{:<36}]", ape6);
    println!("[{:^20}]", pse);
    println!("[{:>30}]", mpsc::RecvError);
    println!("[{:>30}]", mpsc::TryRecvError::Empty);
    println!("[{:>30}]", mpsc::TryRecvError::Disconnected);
    println!("[{:>30}]", mpsc::RecvTimeoutError::Timeout);
    println!("[{:>45}]", mpsc::RecvTimeoutError::Disconnected);
    println!("[{:>30}]", mpsc::SendError(1));
    println!("[{:>30}]", mpsc::TrySendError::Full(1));
    println!("[{:-<30}]", mpsc::TrySendError::Disconnected(1));
    println!("[{:.9}]", mpsc::RecvError);
    let (tx, rx) = mpsc::channel::<i32>();
    drop(rx);
    let e = tx.send(5).unwrap_err();
    println!("[{:>30}] {:?}", e, e);
    let mut args = std::env::args_os();
    args.next();
    println!("{:?}", args);
    let mut args = std::env::args();
    args.next();
    println!("{:?}", args);
}
