use clap::{Parser, Subcommand};

mod config;
mod theme;
mod dbus;
mod ui;
mod app;

use config::Config;
use app::daemon::DaemonClient;

#[derive(Parser)]
#[command(name = "orbit")]
#[command(about = "A WiFi/Bluetooth manager for Wayland")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// List available WiFi networks
    List,
    /// Run as background daemon
    Daemon,
    /// Toggle daemon window visibility
    Toggle {
        /// Optional position override (top-left, top, top-right)
        position: Option<String>,
    },
    /// Output status in JSON format for Waybar
    WaybarStatus,
}

fn main() {
    let cli = Cli::parse();
    
    let config = Config::load();
    
    match cli.command {
        Some(Commands::List) => list_networks(),
        Some(Commands::Daemon) => run_daemon(config),
        Some(Commands::Toggle { position }) => toggle_daemon(position),
        Some(Commands::WaybarStatus) => waybar_status(),
        None => run_gui(config),
    }
}

fn run_gui(config: Config) {
    let app = app::OrbitApp::new(config).expect("Failed to create application");
    app.run();
}

fn run_daemon(config: Config) {
    if DaemonClient::is_daemon_running() {
        eprintln!("Daemon is already running");
        std::process::exit(1);
    }
    
    let app = app::OrbitApp::new_daemon(config).expect("Failed to create daemon");
    app.run();
}

fn toggle_daemon(position: Option<String>) {
    use app::daemon::DaemonCommand;
    
    if !DaemonClient::is_daemon_running() {
        eprintln!("Daemon is not running. Start it with: orbit daemon");
        std::process::exit(1);
    }
    
    match DaemonClient::send_command(DaemonCommand::Toggle(position)) {
        Ok(response) => {
            println!("Daemon response: {}", response);
        }
        Err(e) => {
            eprintln!("Failed to send command: {}", e);
            std::process::exit(1);
        }
    }
}

fn waybar_status() {
    let rt = tokio::runtime::Runtime::new().expect("Failed to create runtime");
    rt.block_on(async {
        let mut status = "Disconnected".to_string();
        if let Ok(nm) = dbus::NetworkManager::new().await {
            if let Ok(aps) = nm.get_access_points().await {
                if let Some(active) = aps.iter().find(|ap| ap.is_connected) {
                    status = active.ssid.clone();
                }
            }
        }
        println!(r#"{{"text": "󱑫", "tooltip": "{}"}}"#, status);
    });
}

fn list_networks() {
    let rt = tokio::runtime::Runtime::new().expect("Failed to create runtime");
    rt.block_on(async {
        match dbus::NetworkManager::new().await {
            Ok(nm) => {
                match nm.get_access_points().await {
                    Ok(aps) => {
                        println!("Available networks:");
                        for ap in aps {
                            let security = match ap.security {
                                dbus::SecurityType::None => "Open",
                                dbus::SecurityType::WEP => "WEP",
                                dbus::SecurityType::WPA => "WPA",
                                dbus::SecurityType::WPA2 => "WPA2",
                                dbus::SecurityType::WPA3 => "WPA3",
                            };
                            let connected = if ap.is_connected { " [Connected]" } else { "" };
                            println!("  {} ({}%) {}{}", ap.ssid, ap.signal_strength, security, connected);
                        }
                    }
                    Err(e) => {
                        eprintln!("Failed to get access points: {}", e);
                    }
                }
            }
            Err(e) => {
                eprintln!("Failed to connect to NetworkManager: {}", e);
            }
        }
    });
}
