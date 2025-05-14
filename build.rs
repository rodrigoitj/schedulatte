// This script sets the application to be GUI only (no console) in release mode
// and embeds the icon in the executable
use std::env;
use std::fs;
use std::path::Path;

fn main() {
    // Copy icons to output directory for both debug and release builds
    let out_dir = env::var("OUT_DIR").unwrap();
    let out_path = Path::new(&out_dir);
    let target_dir = out_path.ancestors().nth(3).unwrap(); // Go up to target/debug or release

    // Copy icons for runtime use
    copy_icon("tray_dark.ico", target_dir);
    copy_icon("tray_light.ico", target_dir);

    println!("Icons copied to {:?}", target_dir);

    // Configure Windows-specific build settings
    #[cfg(windows)]
    {
        let mut res = winres::WindowsResource::new();

        // Embed tray_dark.ico as the executable icon
        res.set_icon("app.ico");

        // Set Windows version info
        res.set("FileDescription", "Schedulatte - Caffeine Scheduler");
        res.set("ProductName", "Schedulatte");
        res.set("FileVersion", env!("CARGO_PKG_VERSION"));
        res.set("ProductVersion", env!("CARGO_PKG_VERSION"));

        // For release builds, set the subsystem to Windows (no console)
        #[cfg(not(debug_assertions))]
        {
            println!("Setting Windows GUI subsystem for release build");
            println!("cargo:rustc-link-arg=/SUBSYSTEM:WINDOWS");
            println!("cargo:rustc-link-arg=/ENTRY:mainCRTStartup");
        }

        // Compile the resource
        if let Err(e) = res.compile() {
            eprintln!("Error compiling resources: {}", e);
            std::process::exit(1);
        }
    }

    #[cfg(not(windows))]
    {
        println!("Building for non-Windows platform");
    }
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
