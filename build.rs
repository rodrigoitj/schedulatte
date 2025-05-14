// This script sets the application to be GUI only (no console) in release mode

#[cfg(all(windows, not(debug_assertions)))]
fn main() {
    // Set Windows subsystem to GUI to hide the console window in release mode
    println!("cargo:rustc-link-arg=/SUBSYSTEM:WINDOWS");
    println!("cargo:rustc-link-arg=/ENTRY:mainCRTStartup");
}

#[cfg(any(not(windows), debug_assertions))]
fn main() {
    // In debug mode or non-Windows platforms, keep console
    println!("Building with console subsystem");
}
