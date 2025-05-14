use chrono::{Local, NaiveTime, Timelike};
use configparser::ini::Ini;
use once_cell::sync::Lazy;
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use sysinfo::System;
use tokio::signal;
use tokio::time::interval;
use windows::core::*;
use windows::Win32::Foundation::*;
use windows::Win32::System::LibraryLoader::*;
use windows::Win32::System::Registry::*;
use windows::Win32::UI::Shell::*;
use windows::Win32::UI::WindowsAndMessaging::*;

struct TimeRange {
    start: NaiveTime,
    end: NaiveTime,
}

struct Config {
    morning: TimeRange,
    afternoon: TimeRange,
}

// Global state for tray
static TRAY_STATE: Lazy<Arc<Mutex<TrayState>>> = Lazy::new(|| {
    Arc::new(Mutex::new(TrayState {
        config: None,
        should_exit: false,
    }))
});

struct TrayState {
    config: Option<Config>,
    should_exit: bool,
}

const WM_USER_TRAY: u32 = WM_USER + 1;
const ID_TRAY_EXIT: u32 = 1001;

// Windows Registry Keys for theme detection
const PERSONALIZE_PATH: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize";
const APPS_USE_LIGHT_THEME: &str = "AppsUseLightTheme";

unsafe extern "system" fn wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_USER_TRAY => {
            if lparam.0 as u32 == WM_RBUTTONUP {
                show_context_menu(hwnd);
            }
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }
        WM_COMMAND => {
            let cmd = (wparam.0 & 0xFFFF) as u32;
            if cmd == ID_TRAY_EXIT {
                let mut state = TRAY_STATE.lock().unwrap();
                state.should_exit = true;
                PostQuitMessage(0);
            }
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }
        WM_DESTROY => {
            PostQuitMessage(0);
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

unsafe fn show_context_menu(hwnd: HWND) {
    let hmenu = CreatePopupMenu().unwrap();

    let state = TRAY_STATE.lock().unwrap();
    if let Some(ref config) = state.config {
        // Add schedule info
        let morning_text = format!(
            "Morning: {:02}:{:02} - {:02}:{:02}",
            config.morning.start.hour(),
            config.morning.start.minute(),
            config.morning.end.hour(),
            config.morning.end.minute()
        );
        let afternoon_text = format!(
            "Afternoon: {:02}:{:02} - {:02}:{:02}",
            config.afternoon.start.hour(),
            config.afternoon.start.minute(),
            config.afternoon.end.hour(),
            config.afternoon.end.minute()
        );
        let caffeine_text = format!(
            "Caffeine: {}",
            if is_caffeine_running() {
                "Active"
            } else {
                "Inactive"
            }
        );

        let _ = AppendMenuW(
            hmenu,
            MF_STRING | MF_GRAYED,
            0,
            &HSTRING::from(morning_text),
        );
        let _ = AppendMenuW(
            hmenu,
            MF_STRING | MF_GRAYED,
            0,
            &HSTRING::from(afternoon_text),
        );
        let _ = AppendMenuW(
            hmenu,
            MF_STRING | MF_GRAYED,
            0,
            &HSTRING::from(caffeine_text),
        );
        let _ = AppendMenuW(hmenu, MF_SEPARATOR, 0, PCWSTR::null());
    }
    drop(state);

    let _ = AppendMenuW(hmenu, MF_STRING, ID_TRAY_EXIT as usize, w!("Exit"));

    let mut pt = POINT::default();
    let _ = GetCursorPos(&mut pt);
    SetForegroundWindow(hwnd);
    TrackPopupMenu(hmenu, TPM_RIGHTBUTTON, pt.x, pt.y, 0, hwnd, None);
    let _ = DestroyMenu(hmenu);
}

fn is_dark_theme() -> bool {
    unsafe {
        let mut hkey = HKEY::default();
        let personalize_path = HSTRING::from(PERSONALIZE_PATH);

        // Open the Registry key
        if RegOpenKeyExW(HKEY_CURRENT_USER, &personalize_path, 0, KEY_READ, &mut hkey).is_err() {
            return false; // Default to light theme if registry access fails
        }

        // Read the value
        let mut buffer = [0u8; 4];
        let mut size = buffer.len() as u32;
        let mut type_val = REG_VALUE_TYPE::default();

        let result = RegQueryValueExW(
            hkey,
            &HSTRING::from(APPS_USE_LIGHT_THEME),
            None,
            Some(&mut type_val),
            Some(buffer.as_mut_ptr()),
            Some(&mut size),
        );

        // Close key
        let _ = RegCloseKey(hkey);

        if result.is_err() || type_val != REG_DWORD {
            return false; // Default to light theme if value read fails
        }

        // AppsUseLightTheme = 0 means dark theme is active
        u32::from_ne_bytes(buffer) == 0
    }
}

fn create_tray_icon(hwnd: HWND) -> std::result::Result<(), Box<dyn std::error::Error>> {
    unsafe {
        // Get the current executable's directory
        let mut buffer = [0u16; 260]; // MAX_PATH
        let len = GetModuleFileNameW(None, &mut buffer);
        let exe_path = String::from_utf16_lossy(&buffer[..len as usize]);
        let exe_dir = std::path::Path::new(&exe_path)
            .parent()
            .unwrap_or(std::path::Path::new(""))
            .to_string_lossy()
            .to_string();

        #[cfg(debug_assertions)]
        println!("Executable directory: {}", exe_dir);

        // Load custom icon based on theme
        let h_instance = GetModuleHandleW(None)?;

        // Determine icon paths - try both relative and absolute
        let icon_name = if is_dark_theme() {
            "tray_dark.ico" // Dark theme icon
        } else {
            "tray_light.ico" // Light theme icon
        };

        // Try different paths to find the icon
        let relative_path = HSTRING::from(icon_name);
        let abs_path = HSTRING::from(format!("{}\\{}", exe_dir, icon_name));

        #[cfg(debug_assertions)]
        {
            println!(
                "Using theme: {}",
                if is_dark_theme() { "dark" } else { "light" }
            );
            println!("Trying icon paths:");
            println!("  - Relative: {}", icon_name);
            println!("  - Absolute: {}", abs_path);
        }

        // Try loading the icon from different locations
        let mut h_icon = LoadImageW(
            h_instance,
            &relative_path,
            IMAGE_ICON,
            0,
            0,
            LR_LOADFROMFILE | LR_DEFAULTSIZE,
        );

        // If relative path fails, try absolute path
        if h_icon.is_err() {
            #[cfg(debug_assertions)]
            println!("Relative path failed, trying absolute path");

            h_icon = LoadImageW(
                h_instance,
                &abs_path,
                IMAGE_ICON,
                0,
                0,
                LR_LOADFROMFILE | LR_DEFAULTSIZE,
            );
        }

        // Choose the icon to use
        let h_icon = if let Ok(icon) = h_icon {
            #[cfg(debug_assertions)]
            println!("Successfully loaded custom icon");
            HICON(icon.0)
        } else {
            #[cfg(debug_assertions)]
            println!("Failed to load custom icon, using system default");
            LoadIconW(HINSTANCE::default(), IDI_APPLICATION)?
        };

        let mut nid = NOTIFYICONDATAW {
            cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
            hWnd: hwnd,
            uID: 1,
            uFlags: NIF_ICON | NIF_MESSAGE | NIF_TIP,
            uCallbackMessage: WM_USER_TRAY,
            hIcon: h_icon,
            ..Default::default()
        };

        let tooltip = "Schedulatte - Caffeine Scheduler";
        let tooltip_wide: Vec<u16> = tooltip.encode_utf16().collect();
        let len = tooltip_wide.len().min(127);
        nid.szTip[..len].copy_from_slice(&tooltip_wide[..len]);

        let result = Shell_NotifyIconW(NIM_ADD, &nid);
        if !result.as_bool() {
            return Err("Failed to create tray icon".into());
        }
        Ok(())
    }
}

fn destroy_tray_icon(hwnd: HWND) -> std::result::Result<(), Box<dyn std::error::Error>> {
    unsafe {
        let nid = NOTIFYICONDATAW {
            cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
            hWnd: hwnd,
            uID: 1,
            ..Default::default()
        };
        let result = Shell_NotifyIconW(NIM_DELETE, &nid);
        if !result.as_bool() {
            return Err("Failed to destroy tray icon".into());
        }
        Ok(())
    }
}

fn run_message_loop() {
    unsafe {
        let instance = GetModuleHandleW(None).unwrap();
        let class_name = w!("SchedulatteTrayClass");

        let wc = WNDCLASSW {
            lpfnWndProc: Some(wnd_proc),
            hInstance: instance.into(),
            lpszClassName: class_name,
            ..Default::default()
        };

        RegisterClassW(&wc);

        let hwnd = CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            class_name,
            w!("Schedulatte"),
            WINDOW_STYLE::default(),
            0,
            0,
            0,
            0,
            None,
            None,
            instance,
            None,
        );

        if let Err(_e) = create_tray_icon(hwnd) {
            #[cfg(debug_assertions)]
            eprintln!("Failed to create tray icon: {}", _e);
            return;
        }

        let mut msg = MSG::default();
        loop {
            let state = TRAY_STATE.lock().unwrap();
            if state.should_exit {
                break;
            }
            drop(state);

            if PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
            thread::sleep(Duration::from_millis(600));
        }

        destroy_tray_icon(hwnd).ok();
        let _ = UnregisterClassW(class_name, instance);
    }
}

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    // Only print to console in debug mode
    #[cfg(debug_assertions)]
    println!("=== Schedulatte Started ===");
    #[cfg(debug_assertions)]
    println!("Loading configuration...");

    let config = load_config("config.ini")?;
    let caffeine_exe = get_caffeine_executable();

    // Set config in global state
    {
        let mut state = TRAY_STATE.lock().unwrap();
        state.config = Some(config);
    }

    // Start tray icon in separate thread
    thread::spawn(|| {
        run_message_loop();
    });

    // Only print to console in debug mode
    #[cfg(debug_assertions)]
    {
        println!("Configuration loaded successfully:");
        let state = TRAY_STATE.lock().unwrap();
        let config = state.config.as_ref().unwrap();
        println!(
            "  Morning: {:02}:{:02} - {:02}:{:02}",
            config.morning.start.hour(),
            config.morning.start.minute(),
            config.morning.end.hour(),
            config.morning.end.minute()
        );
        println!(
            "  Afternoon: {:02}:{:02} - {:02}:{:02}",
            config.afternoon.start.hour(),
            config.afternoon.start.minute(),
            config.afternoon.end.hour(),
            config.afternoon.end.minute()
        );
        drop(state);

        println!("Using executable: {}", caffeine_exe);
        println!("Starting monitoring (checking every 10 minutes)...");
        println!("System tray icon created. Right-click for menu.");
        println!("Press Ctrl+C to stop gracefully\n");
    }

    let mut check_interval = interval(Duration::from_secs(600)); // 10 minutes
    let mut exit_check_interval = interval(Duration::from_millis(100)); // Check exit every 100ms

    // Perform initial check
    {
        let state = TRAY_STATE.lock().unwrap();
        let config = state.config.as_ref().unwrap();
        check_and_manage_caffeine(config, &caffeine_exe).await;
        drop(state);
    }

    loop {
        tokio::select! {
            _ = check_interval.tick() => {
                let state = TRAY_STATE.lock().unwrap();
                if state.should_exit {
                    #[cfg(debug_assertions)]
                    println!("Exit requested from tray menu");
                    break;
                }
                let config = state.config.as_ref().unwrap();
                check_and_manage_caffeine(config, &caffeine_exe).await;
                drop(state);
            }
            _ = exit_check_interval.tick() => {
                let state = TRAY_STATE.lock().unwrap();
                if state.should_exit {
                    #[cfg(debug_assertions)]
                    println!("Exit requested from tray menu");
                    break;
                }
                drop(state);
            }
            _ = signal::ctrl_c() => {
                #[cfg(debug_assertions)]
                println!("\n=== Shutdown Signal Received ===");
                break;
            }
        }
    }

    #[cfg(debug_assertions)]
    println!("Stopping Schedulatte gracefully...");
    if is_caffeine_running() {
        #[cfg(debug_assertions)]
        println!("Stopping caffeine before exit...");
        kill_caffeine();
    }
    #[cfg(debug_assertions)]
    println!("Schedulatte stopped.");

    Ok(())
}

fn load_config(path: &str) -> std::result::Result<Config, Box<dyn std::error::Error>> {
    #[cfg(debug_assertions)]
    println!("Reading config file: {}", path);
    let mut config = Ini::new();
    config.load(path).map_err(|e| {
        #[cfg(debug_assertions)]
        eprintln!("Error loading config file: {}", e);
        e
    })?;

    let morning_start = config
        .get("morning", "start")
        .ok_or("Missing morning start")?;
    let morning_end = config.get("morning", "end").ok_or("Missing morning end")?;
    let afternoon_start = config
        .get("afternoon", "start")
        .ok_or("Missing afternoon start")?;
    let afternoon_end = config
        .get("afternoon", "end")
        .ok_or("Missing afternoon end")?;

    #[cfg(debug_assertions)]
    println!("Parsing time ranges...");
    let morning = parse_time_range(&morning_start, &morning_end)?;
    let afternoon = parse_time_range(&afternoon_start, &afternoon_end)?;

    Ok(Config { morning, afternoon })
}

fn parse_time_range(
    start_str: &str,
    end_str: &str,
) -> std::result::Result<TimeRange, Box<dyn std::error::Error>> {
    let start = NaiveTime::parse_from_str(start_str, "%H:%M")?;
    let end = NaiveTime::parse_from_str(end_str, "%H:%M")?;
    Ok(TimeRange { start, end })
}

fn get_caffeine_executable() -> String {
    if cfg!(target_arch = "x86_64") {
        "caffeine64.exe".to_string()
    } else {
        "caffeine32.exe".to_string()
    }
}

fn is_in_schedule(config: &Config, time: NaiveTime) -> bool {
    is_in_range(&config.morning, time) || is_in_range(&config.afternoon, time)
}

fn is_in_range(range: &TimeRange, time: NaiveTime) -> bool {
    time >= range.start && time <= range.end
}

fn is_caffeine_running() -> bool {
    let mut system = System::new_all();
    system.refresh_processes();

    let mut found_processes = Vec::new();
    for (pid, process) in system.processes() {
        let name = process.name().to_lowercase();
        if name == "caffeine32.exe" || name == "caffeine64.exe" || name == "caffeine.exe" {
            found_processes.push((pid, process.name()));
        }
    }

    let running = !found_processes.is_empty();

    #[cfg(debug_assertions)]
    {
        if running {
            println!("  Found {} caffeine process(es):", found_processes.len());
            for (pid, name) in found_processes {
                println!("    - {} (PID: {})", name, pid);
            }
        } else {
            println!("  No caffeine processes found");
        }
    }

    running
}

fn start_caffeine(executable: &str) {
    #[cfg(debug_assertions)]
    println!("  Attempting to start {}", executable);
    match Command::new(executable).spawn() {
        Ok(_) => {
            #[cfg(debug_assertions)]
            println!("  ✓ Caffeine started successfully")
        }
        Err(_e) => {
            #[cfg(debug_assertions)]
            eprintln!("  ✗ Failed to start caffeine: {}", _e)
        }
    }
}

fn kill_caffeine() {
    #[cfg(debug_assertions)]
    println!("  Searching for caffeine processes to terminate...");
    let mut system = System::new_all();
    system.refresh_processes();

    #[cfg(debug_assertions)]
    let mut found = false;
    for (_pid, process) in system.processes() {
        let name = process.name().to_lowercase();
        if name == "caffeine32.exe" || name == "caffeine64.exe" || name == "caffeine.exe" {
            #[cfg(debug_assertions)]
            {
                found = true;
                println!(
                    "  Found caffeine process: {} (PID: {})",
                    process.name(),
                    _pid
                );
            }
            if !process.kill() {
                #[cfg(debug_assertions)]
                eprintln!("  ✗ Failed to kill caffeine process {}", _pid);
            } else {
                #[cfg(debug_assertions)]
                println!("  ✓ Killed caffeine process {}", _pid);
            }
        }
    }

    #[cfg(debug_assertions)]
    if !found {
        println!("  No caffeine processes found to kill");
    }
}

async fn check_and_manage_caffeine(config: &Config, caffeine_exe: &str) {
    let now = Local::now().time();
    let should_run = is_in_schedule(config, now);
    let is_running = is_caffeine_running();

    #[cfg(debug_assertions)]
    {
        println!("=== Status Check at {} ===", now.format("%H:%M:%S"));
        println!("  Should caffeine be running: {}", should_run);
        println!("  Caffeine currently running: {}", is_running);
    }

    match (should_run, is_running) {
        (true, false) => {
            #[cfg(debug_assertions)]
            println!("  Action: Starting caffeine");
            start_caffeine(caffeine_exe);
        }
        (false, true) => {
            #[cfg(debug_assertions)]
            println!("  Action: Stopping caffeine");
            kill_caffeine();
        }
        (true, true) => {
            #[cfg(debug_assertions)]
            println!("  Action: No action needed (already running)");
        }
        (false, false) => {
            #[cfg(debug_assertions)]
            println!("  Action: No action needed (not scheduled)");
        }
    }

    #[cfg(debug_assertions)]
    println!("  Next check in 10 minutes\n");
}
