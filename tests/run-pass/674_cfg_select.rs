cfg_select! {
    feature = "nope" => { const N: i32 = 1; }
    unix => { const N: i32 = 2; }
    _ => { const N: i32 = 3; }
}
fn main() {
    let m = cfg_select! { target_pointer_width = "64" => { 64 } _ => { 32 } };
    println!("{} {}", N, m);
    cfg_select! {
        windows => { println!("windows"); }
        _ => { let a = 5; println!("stmt {}", a); }
    }
    let s = cfg_select! { windows => "w", _ => "u" };
    println!("{s}");
}
