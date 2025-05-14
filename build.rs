// This script sets the application to be GUI only (no console) in release mode
use std::env;
use std::fs;
use std::path::Path;

#[cfg(all(windows, not(debug_assertions)))]
fn main() {
    // Set Windows subsystem to GUI to hide the console window in release mode
    println!("cargo:rustc-link-arg=/SUBSYSTEM:WINDOWS");
    println!("cargo:rustc-link-arg=/ENTRY:mainCRTStartup");

    // Copy icons to output directory
    let out_dir = env::var("OUT_DIR").unwrap();
    let out_path = Path::new(&out_dir);
    let target_dir = out_path.ancestors().nth(3).unwrap(); // Go up to target/release

    // Copy icons
    copy_icon("tray_dark.ico", target_dir);
    copy_icon("tray_light.ico", target_dir);

    println!("Icons copied to {:?}", target_dir);
}

#[cfg(debug_assertions)]
fn main() {
    // In debug mode, keep console
    println!("Building with console subsystem");

    // Copy icons to output directory for debug build too
    let out_dir = env::var("OUT_DIR").unwrap();
    let out_path = Path::new(&out_dir);
    let target_dir = out_path.ancestors().nth(3).unwrap(); // Go up to target/debug

    // Copy icons
    copy_icon("tray_dark.ico", target_dir);
    copy_icon("tray_light.ico", target_dir);

    println!("Icons copied to {:?}", target_dir);
}

#[cfg(not(windows))]
fn main() {
    println!("Building for non-Windows platform");
}

fn copy_icon(icon_name: &str, target_dir: &Path) {
    let source_path = icon_name;
    let dest_path = target_dir.join(icon_name);

    if Path::new(source_path).exists() {
        match fs::copy(source_path, &dest_path) {
            Ok(_) => println!("Copied {} to {:?}", icon_name, dest_path),
            Err(e) => eprintln!("Failed to copy {}: {}", icon_name, e),
        }
    } else {
        eprintln!("Icon file not found: {}", source_path);
    }
}
