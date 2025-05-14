use chrono::{Local, NaiveTime, Timelike};
use configparser::ini::Ini;
use std::process::Command;
use std::time::Duration;
use sysinfo::System;
use tokio::time::interval;
use tokio::signal;

struct TimeRange {
    start: NaiveTime,
    end: NaiveTime,
}

struct Config {
    morning: TimeRange,
    afternoon: TimeRange,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Schedulatte Started ===");
    println!("Loading configuration...");
    
    let config = load_config("config.ini")?;
    let caffeine_exe = get_caffeine_executable();
    
    println!("Configuration loaded successfully:");
    println!("  Morning: {:02}:{:02} - {:02}:{:02}", 
             config.morning.start.hour(), config.morning.start.minute(),
             config.morning.end.hour(), config.morning.end.minute());
    println!("  Afternoon: {:02}:{:02} - {:02}:{:02}", 
             config.afternoon.start.hour(), config.afternoon.start.minute(),
             config.afternoon.end.hour(), config.afternoon.end.minute());
    println!("Using executable: {}", caffeine_exe);
    println!("Starting monitoring (checking every 10 minutes)...");
    println!("Press Ctrl+C to stop gracefully\n");
    
    let mut interval = interval(Duration::from_secs(600)); // 10 minutes
    
    // Perform initial check
    check_and_manage_caffeine(&config, &caffeine_exe).await;
    
    loop {
        tokio::select! {
            _ = interval.tick() => {
                check_and_manage_caffeine(&config, &caffeine_exe).await;
            }
            _ = signal::ctrl_c() => {
                println!("\n=== Shutdown Signal Received ===");
                println!("Stopping Schedulatte gracefully...");
                
                // Optionally kill caffeine on shutdown
                if is_caffeine_running() {
                    println!("Stopping caffeine before exit...");
                    kill_caffeine();
                }
                
                println!("Schedulatte stopped.");
                break;
            }
        }
    }
    
    Ok(())
}

async fn check_and_manage_caffeine(config: &Config, caffeine_exe: &str) {
    let now = Local::now().time();
    let should_run = is_in_schedule(config, now);
    let is_running = is_caffeine_running();
    
    println!("=== Status Check at {} ===", now.format("%H:%M:%S"));
    println!("  Should caffeine be running: {}", should_run);
    println!("  Caffeine currently running: {}", is_running);
    
    match (should_run, is_running) {
        (true, false) => {
            println!("  Action: Starting caffeine");
            start_caffeine(caffeine_exe);
        }
        (false, true) => {
            println!("  Action: Stopping caffeine");
            kill_caffeine();
        }
        (true, true) => {
            println!("  Action: No action needed (already running)");
        }
        (false, false) => {
            println!("  Action: No action needed (not scheduled)");
        }
    }
    
    println!("  Next check in 10 minutes\n");
}

fn load_config(path: &str) -> Result<Config, Box<dyn std::error::Error>> {
    println!("Reading config file: {}", path);
    let mut config = Ini::new();
    config.load(path).map_err(|e| {
        eprintln!("Error loading config file: {}", e);
        e
    })?;
    
    let morning_start = config.get("morning", "start").ok_or("Missing morning start")?;
    let morning_end = config.get("morning", "end").ok_or("Missing morning end")?;
    let afternoon_start = config.get("afternoon", "start").ok_or("Missing afternoon start")?;
    let afternoon_end = config.get("afternoon", "end").ok_or("Missing afternoon end")?;
    
    println!("Parsing time ranges...");
    let morning = parse_time_range(&morning_start, &morning_end)?;
    let afternoon = parse_time_range(&afternoon_start, &afternoon_end)?;
    
    Ok(Config { morning, afternoon })
}

fn parse_time_range(start_str: &str, end_str: &str) -> Result<TimeRange, Box<dyn std::error::Error>> {
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
    
    if running {
        println!("  Found {} caffeine process(es):", found_processes.len());
        for (pid, name) in found_processes {
            println!("    - {} (PID: {})", name, pid);
        }
    } else {
        println!("  No caffeine processes found");
    }
    
    running
}

fn start_caffeine(executable: &str) {
    println!("  Attempting to start {}", executable);
    match Command::new(executable).spawn() {
        Ok(_) => println!("  ✓ Caffeine started successfully"),
        Err(e) => eprintln!("  ✗ Failed to start caffeine: {}", e),
    }
}

fn kill_caffeine() {
    println!("  Searching for caffeine processes to terminate...");
    let mut system = System::new_all();
    system.refresh_processes();
    
    let mut found_processes = false;
    for (pid, process) in system.processes() {
        let name = process.name().to_lowercase();
        if name == "caffeine32.exe" || name == "caffeine64.exe" || name == "caffeine.exe" {
            found_processes = true;
            println!("  Found caffeine process: {} (PID: {})", process.name(), pid);
            if !process.kill() {
                eprintln!("  ✗ Failed to kill caffeine process {}", pid);
            } else {
                println!("  ✓ Killed caffeine process {}", pid);
            }
        }
    }
    
    if !found_processes {
        println!("  No caffeine processes found to kill");
    }
} 