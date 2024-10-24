#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn mobile_entry_point() {
    main();
}

#[allow(dead_code)]
fn main() {
    // This function is defined here to satisfy the compiler,
    // but the actual implementation is in main.rs
}