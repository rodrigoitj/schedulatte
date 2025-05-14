# Schedulatte

A Rust-based application that automatically manages the Caffeine keep-awake utility based on configurable time schedules.

## What it does

Schedulatte monitors your system and automatically:

- Starts Caffeine during your configured active hours
- Stops Caffeine outside of scheduled times
- Checks every 10 minutes to ensure Caffeine is running when it should be
- Handles system restarts gracefully by checking current state vs desired state

## Prerequisites

⚠️ **Important**: This application does **NOT** include the Caffeine executables. You must download and provide them separately.

### Required Files

1. **Caffeine Executables**: Download from [Caffeine official website](http://www.zhornsoftware.co.uk/caffeine/)

   - `caffeine32.exe` (for 32-bit systems)
   - `caffeine64.exe` (for 64-bit systems)
   - Place these files in the same directory as the Schedulatte executable

2. **Config File**: Create a `config.ini` file (see Configuration section below)

## Installation

1. Download or build the schedulatte executable
2. Place it in a directory of your choice
3. Download Caffeine executables and place them in the same directory
4. Create your `config.ini` file

## Configuration

Create a `config.ini` file in the same directory as the executable:

```ini
[morning]
start = 08:30
end = 12:00

[afternoon]
start = 13:00
end = 18:00
```

### Configuration Options

- **Time Format**: Use 24-hour format (HH:MM)
- **Morning Section**: Define morning work hours
- **Afternoon Section**: Define afternoon work hours
- **Multiple Periods**: The app supports two time periods per day

### Example Configurations

**Standard Work Day:**

```ini
[morning]
start = 09:00
end = 12:30

[afternoon]
start = 13:30
end = 17:30
```

**Extended Hours:**

```ini
[morning]
start = 07:00
end = 11:00

[afternoon]
start = 12:00
end = 20:00
```

## Usage

### Running the Application

**Development Mode:**

```bash
cargo run
```

**Release Mode:**

```bash
# Build release version
cargo build --release

# Run the executable
./target/release/schedulatte.exe
```

### Stopping the Application

Press `Ctrl+C` to stop Schedulatte gracefully. The application will:

1. Show a shutdown message
2. Stop any running Caffeine processes (optional)
3. Exit cleanly

## Features

- **Automatic Architecture Detection**: Selects caffeine32.exe or caffeine64.exe based on system architecture
- **Smart State Management**: Only starts/stops Caffeine when necessary
- **Detailed Logging**: Shows all actions and status checks
- **Graceful Shutdown**: Handles Ctrl+C properly
- **Process Management**: Accurately detects and manages Caffeine processes
- **Robust Error Handling**: Continues running even if individual operations fail

## Directory Structure
