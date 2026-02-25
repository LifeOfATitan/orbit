use gtk4::{Application, glib};
use gtk4::prelude::*;
use gtk4::gio::ApplicationFlags;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

pub mod daemon;

use crate::config::Config;
use crate::theme::Theme;
use crate::dbus::{NetworkManager, BluetoothManager};
use crate::dbus::network_manager::{AccessPoint, SecurityType, SavedNetwork, NetworkDetails};
use crate::dbus::bluez::BluetoothDevice;
use crate::ui::{OrbitWindow, DeviceAction};
use daemon::{DaemonServer, DaemonCommand};

enum AppEvent {
    WifiScanResult(Vec<AccessPoint>),
    SavedNetworksResult(Vec<SavedNetwork>),
    NetworkDetailsResult(NetworkDetails),
    BtScanResult(Vec<BluetoothDevice>),
    WifiPowerState(bool),
    BtPowerState(bool),
    ConnectStarted(String),
    ConnectSuccess,
    ConnectHidden(String, String),
    BtActionStarted(String, DeviceAction),
    BtActionComplete,
    BtUnavailable,
    Error(String),
    Notify(String),
    CaptivePortal(String),
    DaemonCommand(DaemonCommand),
}

pub struct OrbitApp {
    app: Application,
    config: Config,
    theme: Rc<RefCell<Theme>>,
    is_daemon: bool,
}

impl OrbitApp {
    pub fn new(config: Config) -> Result<Self, glib::Error> {
        Self::new_with_mode(config, false)
    }
    
    pub fn new_daemon(config: Config) -> Result<Self, glib::Error> {
        Self::new_with_mode(config, true)
    }
    
    fn new_with_mode(config: Config, is_daemon: bool) -> Result<Self, glib::Error> {
        let app = Application::new(Some("com.orbit.app"), ApplicationFlags::empty());
        
        let theme = Theme::load();
        let theme = Rc::new(RefCell::new(theme));
        
        Ok(Self {
            app,
            config,
            theme,
            is_daemon,
        })
    }
    
    pub fn run(&self) -> glib::ExitCode {
        let config = self.config.clone();
        let win_theme = self.theme.clone();
        let is_daemon = self.is_daemon;
        
        self.app.connect_activate(move |app| {
            let config = config.clone();
            let win_theme = win_theme.clone();
            
            let rt = Arc::new(tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime"));
            let win = OrbitWindow::new(app, config, win_theme.clone());
            
            let nm: Arc<Mutex<Option<NetworkManager>>> = Arc::new(Mutex::new(None));
            let bt: Arc<Mutex<Option<BluetoothManager>>> = Arc::new(Mutex::new(None));
            
            let (tx, rx) = async_channel::unbounded::<AppEvent>();
            
            // Initialization thread
            {
                let rt = rt.clone();
                let nm_arc = nm.clone();
                let bt_arc = bt.clone();
                let tx = tx.clone();
                
                std::thread::spawn(move || {
                    let mut nm_inst = None;
                    for i in 0..5 {
                        if let Ok(inst) = rt.block_on(async { NetworkManager::new().await }) {
                            nm_inst = Some(inst);
                            break;
                        }
                        if i < 4 {
                            std::thread::sleep(std::time::Duration::from_secs(1));
                        }
                    }

                    let mut bt_inst = None;
                    for i in 0..5 {
                        if let Ok(inst) = rt.block_on(async { BluetoothManager::new().await }) {
                            bt_inst = Some(inst);
                            break;
                        }
                        if i < 4 {
                            std::thread::sleep(std::time::Duration::from_secs(1));
                        }
                    }
                    
                    if let Some(ref nm) = nm_inst {
                        if let Ok(enabled) = rt.block_on(async { nm.is_wifi_enabled().await }) {
                            let _ = tx.send_blocking(AppEvent::WifiPowerState(enabled));
                            
                            if enabled {
                                let active = rt.block_on(async { nm.get_active_ssid().await });
                                let connected_ssid = if let Some(ref ssid) = active {
                                    // Already connected (autoconnect worked), notify
                                    let _ = tx.send_blocking(AppEvent::Notify(
                                        format!("Connected to {}", ssid)
                                    ));
                                    Some(ssid.clone())
                                } else {
                                    // Not connected yet — wait for NM autoconnect to kick in
                                    std::thread::sleep(std::time::Duration::from_secs(4));
                                    let active_after = rt.block_on(async { nm.get_active_ssid().await });
                                    if let Some(ref ssid) = active_after {
                                        let _ = tx.send_blocking(AppEvent::Notify(
                                            format!("Connected to {}", ssid)
                                        ));
                                        Some(ssid.clone())
                                    } else {
                                        // Still not connected, trigger a scan
                                        let _ = rt.block_on(async { nm.scan().await });
                                        None
                                    }
                                };
                                // Check for captive portal on autoconnected network
                                if let Some(ssid) = connected_ssid {
                                    std::thread::sleep(std::time::Duration::from_secs(2));
                                    if let Ok(connectivity) = rt.block_on(async { nm.check_connectivity().await }) {
                                        if connectivity == 2 {
                                            let _ = tx.send_blocking(AppEvent::CaptivePortal(ssid));
                                        }
                                    }
                                }
                            }
                        }
                        if let Ok(aps) = rt.block_on(async { nm.get_access_points().await }) {
                            let _ = tx.send_blocking(AppEvent::WifiScanResult(aps));
                        }
                        if let Ok(saved) = rt.block_on(async { nm.get_saved_networks().await }) {
                            let _ = tx.send_blocking(AppEvent::SavedNetworksResult(saved));
                        }
                    }
                    
                    if let Some(ref bt) = bt_inst {
                        if !rt.block_on(async { bt.is_available().await }) {
                            let _ = tx.send_blocking(AppEvent::BtUnavailable);
                        } else {
                            if let Ok(devices) = rt.block_on(async { bt.get_devices().await }) {
                                let _ = tx.send_blocking(AppEvent::BtScanResult(devices));
                            }
                            if let Ok(powered) = rt.block_on(async { bt.is_powered().await }) {
                                let _ = tx.send_blocking(AppEvent::BtPowerState(powered));
                            }
                        }
                    } else {
                        let _ = tx.send_blocking(AppEvent::BtUnavailable);
                    }
                    
                    *nm_arc.lock().unwrap() = nm_inst;
                    *bt_arc.lock().unwrap() = bt_inst;
                });
            }
            
            let is_visible = Rc::new(RefCell::new(!is_daemon));
            
            // Sync is_visible with actual window visibility (handles auto-close, escape, etc.)
            let is_visible_sync = is_visible.clone();
            win.window().connect_notify_local(Some("visible"), move |window, _| {
                *is_visible_sync.borrow_mut() = window.is_visible();
            });
            
            if !is_daemon {
                win.show();
            }
            
            setup_events_receiver(win.clone(), rx.clone(), is_visible.clone(), nm.clone(), bt.clone(), rt.clone(), tx.clone(), win_theme.clone());
            setup_ui_callbacks(win.clone(), nm.clone(), bt.clone(), rt.clone(), tx.clone());
            setup_periodic_refresh(win.clone(), nm, bt, rt, tx.clone(), is_visible.clone());
            
            if is_daemon {
                match DaemonServer::new() {
                    Ok(server) => {
                        server.run(move |cmd| {
                            let _ = tx.send_blocking(AppEvent::DaemonCommand(cmd));
                        });
                    }
                    Err(_) => {}
                }
            }
        });
        
        // Run without arguments to prevent GTK from parsing subcommands as files
        self.app.run_with_args(&[] as &[&str])
    }
}

fn setup_events_receiver(
    win: OrbitWindow,
    rx: async_channel::Receiver<AppEvent>,
    is_visible: Rc<RefCell<bool>>,
    nm: Arc<Mutex<Option<NetworkManager>>>,
    bt: Arc<Mutex<Option<BluetoothManager>>>,
    rt: Arc<tokio::runtime::Runtime>,
    tx: async_channel::Sender<AppEvent>,
    win_theme: Rc<RefCell<Theme>>,
) {
    glib::spawn_future_local(async move {
        while let Ok(event) = rx.recv().await {
            match event {
                AppEvent::WifiScanResult(aps) => {
                    win.network_list().set_networks(aps);
                }
                AppEvent::SavedNetworksResult(networks) => {
                    win.saved_networks_list().set_networks(networks);
                }
                AppEvent::NetworkDetailsResult(details) => {
                    win.show_network_details(&details);
                }
                AppEvent::BtScanResult(devices) => {
                    win.device_list().set_devices(devices);
                }
                AppEvent::WifiPowerState(enabled) => {
                    // Only update switch if on WiFi or Saved tab
                    if let Some(tab) = win.stack().visible_child_name() {
                        let tab_str = tab.as_str();
                        if tab_str == "wifi" || tab_str == "saved" {
                            win.header().set_power_state(enabled);
                        }
                    }
                }
                AppEvent::BtPowerState(enabled) => {
                    // Only update switch if on Bluetooth tab
                    if let Some(tab) = win.stack().visible_child_name() {
                        let tab_str = tab.as_str();
                        if tab_str == "bluetooth" {
                            win.header().set_power_state(enabled);
                        }
                    }
                }
                AppEvent::Error(msg) => {
                    win.network_list().set_connecting_ssid(None);
                    win.show_error(&msg);
                }
                AppEvent::Notify(msg) => {
                    std::thread::spawn(move || {
                        let _ = std::process::Command::new("notify-send")
                            .arg("Orbit")
                            .arg(&msg)
                            .arg("--app-name=Orbit")
                            .arg("-i")
                            .arg("network-wireless")
                            .spawn();
                    });
                }
                AppEvent::CaptivePortal(ssid) => {
                    std::thread::spawn(move || {
                        let _ = std::process::Command::new("notify-send")
                            .arg("Orbit")
                            .arg(&format!("Captive portal detected on {} — opening login page...", ssid))
                            .arg("--app-name=Orbit")
                            .arg("-i")
                            .arg("network-wireless")
                            .spawn();
                        let _ = std::process::Command::new("xdg-open")
                            .arg("http://neverssl.com")
                            .spawn();
                    });
                }
                AppEvent::ConnectStarted(ssid) => {
                    win.network_list().set_connecting_ssid(Some(ssid));
                }
                AppEvent::ConnectSuccess => {
                    win.network_list().set_connecting_ssid(None);
                    win.hide_password_dialog();
                }
                AppEvent::ConnectHidden(ssid, password) => {
                    let nm_ref = nm.clone();
                    let rt_ref = rt.clone();
                    let tx_ref = tx.clone();
                    
                    std::thread::spawn(move || {
                        let nm_guard = nm_ref.lock().unwrap();
                        if let Some(ref nm_inst) = *nm_guard {
                            // Find a physical wireless device
                            match rt_ref.block_on(async { nm_inst.get_wireless_devices().await }) {
                                Ok(devices) => {
                                    if let Some(device_path) = devices.get(0) {
                                        let pwd = if password.is_empty() { None } else { Some(password.as_str()) };
                                        match rt_ref.block_on(async { nm_inst.connect_hidden(&ssid, pwd, device_path).await }) {
                                            Ok(()) => {
                                                let _ = tx_ref.send_blocking(AppEvent::ConnectSuccess);
                                                let _ = tx_ref.send_blocking(AppEvent::Notify(format!("Connecting to hidden network {}...", ssid)));
                                            }
                                            Err(e) => {
                                                let _ = tx_ref.send_blocking(AppEvent::Error(format!("Hidden connect failed: {}", e)));
                                            }
                                        }
                                    } else {
                                        let _ = tx_ref.send_blocking(AppEvent::Error("No WiFi device found".to_string()));
                                    }
                                }
                                Err(e) => {
                                    let _ = tx_ref.send_blocking(AppEvent::Error(format!("Failed to query WiFi devices: {}", e)));
                                }
                            }
                        }
                    });
                }
                AppEvent::BtActionStarted(path, action) => {
                    win.device_list().set_action_state(Some(path), Some(action));
                }
                AppEvent::BtActionComplete => {
                    win.device_list().set_action_state(None, None);
                }
                AppEvent::BtUnavailable => {
                    win.header().bluetooth_tab().set_sensitive(false);
                    win.device_list().show_no_adapter();
                }
                AppEvent::DaemonCommand(cmd) => {
                    match cmd {
                        DaemonCommand::Show => {
                            win.show();
                            *is_visible.borrow_mut() = true;
                            // Trigger refresh
                            let nm_ref = nm.clone();
                            let bt_ref = bt.clone();
                            let rt_ref = rt.clone();
                            let tx_ref = tx.clone();
                            std::thread::spawn(move || {
                                let nm_guard = nm_ref.lock().unwrap();
                                if let Some(ref nm_inst) = *nm_guard {
                                    if let Ok(enabled) = rt_ref.block_on(async { nm_inst.is_wifi_enabled().await }) {
                                        let _ = tx_ref.send_blocking(AppEvent::WifiPowerState(enabled));
                                    }
                                    if let Ok(aps) = rt_ref.block_on(async { nm_inst.get_access_points().await }) {
                                        let _ = tx_ref.send_blocking(AppEvent::WifiScanResult(aps));
                                    }
                                }
                                let bt_guard = bt_ref.lock().unwrap();
                                if let Some(ref bt_inst) = *bt_guard {
                                    if let Ok(powered) = rt_ref.block_on(async { bt_inst.is_powered().await }) {
                                        let _ = tx_ref.send_blocking(AppEvent::BtPowerState(powered));
                                    }
                                    if let Ok(devices) = rt_ref.block_on(async { bt_inst.get_devices().await }) {
                                        let _ = tx_ref.send_blocking(AppEvent::BtScanResult(devices));
                                    }
                                }
                            });
                        }
                        DaemonCommand::Hide => {
                            win.hide();
                            *is_visible.borrow_mut() = false;
                        }
                        DaemonCommand::Toggle(position) => {
                            if *is_visible.borrow() {
                                win.hide();
                                *is_visible.borrow_mut() = false;
                            } else {
                                if let Some(pos) = position {
                                    win.set_position(&pos);
                                }
                                win.show();
                                *is_visible.borrow_mut() = true;
                                // Trigger refresh
                                let nm_ref = nm.clone();
                                let bt_ref = bt.clone();
                                let rt_ref = rt.clone();
                                let tx_ref = tx.clone();
                                std::thread::spawn(move || {
                                    let nm_guard = nm_ref.lock().unwrap();
                                    if let Some(ref nm_inst) = *nm_guard {
                                        if let Ok(enabled) = rt_ref.block_on(async { nm_inst.is_wifi_enabled().await }) {
                                            let _ = tx_ref.send_blocking(AppEvent::WifiPowerState(enabled));
                                        }
                                        if let Ok(aps) = rt_ref.block_on(async { nm_inst.get_access_points().await }) {
                                            let _ = tx_ref.send_blocking(AppEvent::WifiScanResult(aps));
                                        }
                                    }
                                    let bt_guard = bt_ref.lock().unwrap();
                                    if let Some(ref bt_inst) = *bt_guard {
                                        if let Ok(powered) = rt_ref.block_on(async { bt_inst.is_powered().await }) {
                                            let _ = tx_ref.send_blocking(AppEvent::BtPowerState(powered));
                                        }
                                        if let Ok(devices) = rt_ref.block_on(async { bt_inst.get_devices().await }) {
                                            let _ = tx_ref.send_blocking(AppEvent::BtScanResult(devices));
                                        }
                                    }
                                });
                            }
                        }
                        DaemonCommand::ReloadTheme => {
                            let new_theme = Theme::load();
                            *win_theme.borrow_mut() = new_theme;
                            win.apply_theme();
                        }
                        DaemonCommand::ReloadConfig => {
                            win.reload_config();
                        }
                        DaemonCommand::Quit => {
                            std::process::exit(0);
                        }
                    }
                }

            }
        }
    });
}

fn setup_ui_callbacks(
    win: OrbitWindow,
    nm: Arc<Mutex<Option<NetworkManager>>>,
    bt: Arc<Mutex<Option<BluetoothManager>>>,
    rt: Arc<tokio::runtime::Runtime>,
    tx: async_channel::Sender<AppEvent>,
) {
    // Tab switching
    let stack = win.stack().clone();
    let header = win.header().clone();

    let stack_wifi = stack.clone();
    let header_wifi = header.clone();
    let nm_wifi = nm.clone();
    let rt_wifi = rt.clone();
    let tx_wifi = tx.clone();
    header.wifi_tab().connect_clicked(move |_| {
        stack_wifi.set_visible_child_name("wifi");
        header_wifi.set_tab("wifi");
        let nm = nm_wifi.clone();
        let rt = rt_wifi.clone();
        let tx = tx_wifi.clone();
        std::thread::spawn(move || {
            let nm_guard = nm.lock().unwrap();
            if let Some(ref nm_inst) = *nm_guard {
                if let Ok(enabled) = rt.block_on(async { nm_inst.is_wifi_enabled().await }) {
                    let _ = tx.send_blocking(AppEvent::WifiPowerState(enabled));
                }
            }
        });
    });

    let stack_saved = stack.clone();
    let header_saved = header.clone();
    let nm_saved = nm.clone();
    let rt_saved = rt.clone();
    let tx_saved = tx.clone();
    header.saved_tab().connect_clicked(move |_| {
        stack_saved.set_visible_child_name("saved");
        header_saved.set_tab("saved");
        let nm = nm_saved.clone();
        let rt = rt_saved.clone();
        let tx = tx_saved.clone();
        std::thread::spawn(move || {
            let nm_guard = nm.lock().unwrap();
            if let Some(ref nm_inst) = *nm_guard {
                if let Ok(saved) = rt.block_on(async { nm_inst.get_saved_networks().await }) {
                    let _ = tx.send_blocking(AppEvent::SavedNetworksResult(saved));
                }
            }
        });
    });

    let stack_bt = stack.clone();
    let header_bt = header.clone();
    let bt_tab = bt.clone();
    let rt_bt_tab = rt.clone();
    let tx_bt_tab = tx.clone();
    header.bluetooth_tab().connect_clicked(move |_| {
        stack_bt.set_visible_child_name("bluetooth");
        header_bt.set_tab("bluetooth");
        let bt = bt_tab.clone();
        let rt = rt_bt_tab.clone();
        let tx = tx_bt_tab.clone();
        std::thread::spawn(move || {
            let bt_guard = bt.lock().unwrap();
            if let Some(ref bt_inst) = *bt_guard {
                if let Ok(powered) = rt.block_on(async { bt_inst.is_powered().await }) {
                    let _ = tx.send_blocking(AppEvent::BtPowerState(powered));
                }
            }
        });
    });
    
    // WiFi Scan
    let nm_scan = nm.clone();
    let rt_scan = rt.clone();
    let tx_scan = tx.clone();
    let net_list = win.network_list().clone();
    win.network_list().scan_button().connect_clicked(move |_| {
        net_list.show_scanning();
        let nm = nm_scan.clone();
        let rt = rt_scan.clone();
        let tx = tx_scan.clone();
        std::thread::spawn(move || {
            let nm_guard = nm.lock().unwrap();
            if let Some(ref nm_inst) = *nm_guard {
                let _ = rt.block_on(async { nm_inst.scan().await });
                std::thread::sleep(std::time::Duration::from_secs(2));
                if let Ok(aps) = rt.block_on(async { nm_inst.get_access_points().await }) {
                    let _ = tx.send_blocking(AppEvent::WifiScanResult(aps));
                }
            }
        });
    });

    // Saved Networks - Refresh
    let nm_saved_refresh = nm.clone();
    let rt_saved_refresh = rt.clone();
    let tx_saved_refresh = tx.clone();
    win.saved_networks_list().refresh_button().connect_clicked(move |_| {
        let nm = nm_saved_refresh.clone();
        let rt = rt_saved_refresh.clone();
        let tx = tx_saved_refresh.clone();
        std::thread::spawn(move || {
            let nm_guard = nm.lock().unwrap();
            if let Some(ref nm_inst) = *nm_guard {
                if let Ok(saved) = rt.block_on(async { nm_inst.get_saved_networks().await }) {
                    let _ = tx.send_blocking(AppEvent::SavedNetworksResult(saved));
                }
            }
        });
    });

    // Saved Networks - Forget
    let nm_forget = nm.clone();
    let rt_forget = rt.clone();
    let tx_forget = tx.clone();
    win.saved_networks_list().set_on_forget(move |path: String| {
        let nm = nm_forget.clone();
        let rt = rt_forget.clone();
        let tx = tx_forget.clone();
        std::thread::spawn(move || {
            let nm_guard = nm.lock().unwrap();
            if let Some(ref nm_inst) = *nm_guard {
                if let Ok(()) = rt.block_on(async { nm_inst.forget_network(&path).await }) {
                    if let Ok(saved) = rt.block_on(async { nm_inst.get_saved_networks().await }) {
                        let _ = tx.send_blocking(AppEvent::SavedNetworksResult(saved));
                    }
                }
            }
        });
    });

    // Saved Networks - Autoconnect Toggle
    let nm_autoconnect = nm.clone();
    let rt_autoconnect = rt.clone();
    let tx_autoconnect = tx.clone();
    win.saved_networks_list().set_on_autoconnect_toggle(move |path: String, enabled: bool| {
        let nm = nm_autoconnect.clone();
        let rt = rt_autoconnect.clone();
        let tx = tx_autoconnect.clone();
        std::thread::spawn(move || {
            let nm_guard = nm.lock().unwrap();
            if let Some(ref nm_inst) = *nm_guard {
                match rt.block_on(async { nm_inst.set_autoconnect(&path, enabled).await }) {
                    Ok(()) => {
                        std::thread::sleep(std::time::Duration::from_millis(100));
                        match rt.block_on(async { nm_inst.get_saved_networks().await }) {
                            Ok(saved) => {
                                let _ = tx.send_blocking(AppEvent::SavedNetworksResult(saved));
                            }
                            Err(e) => {
                                let _ = tx.send_blocking(AppEvent::Error(format!("Failed to refresh: {}", e)));
                            }
                        }
                    }
                    Err(e) => {
                        let _ = tx.send_blocking(AppEvent::Error(format!("Failed to update autoconnect: {}", e)));
                        if let Ok(saved) = rt.block_on(async { nm_inst.get_saved_networks().await }) {
                            let _ = tx.send_blocking(AppEvent::SavedNetworksResult(saved));
                        }
                    }
                }
            }
        });
    });
    
    // WiFi Connect
    let nm_conn = nm.clone();
    let rt_conn = rt.clone();
    let tx_conn = tx.clone();
    let win_conn = win.clone();
    let tx_conn_hidden = tx.clone();
    let win_conn_hidden = win.clone();
    win.network_list().set_on_connect_hidden(move || {
        let tx = tx_conn_hidden.clone();
        win_conn_hidden.show_hidden_dialog(move |data| {
            if let Some((ssid, password)) = data {
                let _ = tx.send_blocking(AppEvent::ConnectHidden(ssid, password));
            }
        });
    });

    win.network_list().set_on_connect(move |ap: AccessPoint| {
        let nm = nm_conn.clone();
        let rt = rt_conn.clone();
        let tx = tx_conn.clone();
        let ap_path = ap.device_path.clone();
        let ssid = ap.ssid.clone();
        
        if ap.is_connected {
            std::thread::spawn(move || {
                let nm_guard = nm.lock().unwrap();
                if let Some(ref nm_inst) = *nm_guard {
                    match rt.block_on(async { nm_inst.disconnect().await }) {
                        Ok(()) => {
                            if let Ok(aps) = rt.block_on(async { nm_inst.get_access_points().await }) {
                                let _ = tx.send_blocking(AppEvent::WifiScanResult(aps));
                            }
                        }
                        Err(e) => {
                            let _ = tx.send_blocking(AppEvent::Error(format!("Disconnect failed: {}", e)));
                        }
                    }
                }
            });
        } else {
            if ap.security != SecurityType::None {
                let tx_dialog = tx.clone();
                let nm_dialog = nm.clone();
                let rt_dialog = rt.clone();
                let ssid_dialog = ssid.clone();
                let ap_path_dialog = ap_path.clone();
                win_conn.show_password_dialog(&ssid, move |password| {
                    if let Some(pwd) = password {
                        let tx = tx_dialog.clone();
                        let nm = nm_dialog.clone();
                        let rt = rt_dialog.clone();
                        let ssid = ssid_dialog.clone();
                        let ap_path = ap_path_dialog.clone();
                        std::thread::spawn(move || {
                            let nm_guard = nm.lock().unwrap();
                            if let Some(ref nm_inst) = *nm_guard {
                                match rt.block_on(async { nm_inst.connect(&ssid, Some(&pwd), &ap_path).await }) {
                                    Ok(()) => {
                                        let _ = tx.send_blocking(AppEvent::ConnectSuccess);
                                        let _ = tx.send_blocking(AppEvent::Notify(
                                            format!("Connected to {}", ssid)
                                        ));
                                        if let Ok(aps) = rt.block_on(async { nm_inst.get_access_points().await }) {
                                            let _ = tx.send_blocking(AppEvent::WifiScanResult(aps));
                                        }
                                        // Check for captive portal
                                        std::thread::sleep(std::time::Duration::from_secs(2));
                                        if let Ok(connectivity) = rt.block_on(async { nm_inst.check_connectivity().await }) {
                                            if connectivity == 2 {
                                                let _ = tx.send_blocking(AppEvent::CaptivePortal(ssid));
                                            }
                                        }
                                    }
                                    Err(e) => { let _ = tx.send_blocking(AppEvent::Error(format!("Connect failed: {}", e))); }
                                }
                            }
                        });
                    }
                });
            } else {
                let _ = tx.send_blocking(AppEvent::ConnectStarted(ssid.clone()));
                std::thread::spawn(move || {
                    let nm_guard = nm.lock().unwrap();
                    if let Some(ref nm_inst) = *nm_guard {
                        match rt.block_on(async { nm_inst.connect(&ssid, None, &ap_path).await }) {
                            Ok(()) => {
                                let _ = tx.send_blocking(AppEvent::ConnectSuccess);
                                let _ = tx.send_blocking(AppEvent::Notify(
                                    format!("Connected to {}", ssid)
                                ));
                                if let Ok(aps) = rt.block_on(async { nm_inst.get_access_points().await }) {
                                    let _ = tx.send_blocking(AppEvent::WifiScanResult(aps));
                                }
                                // Check for captive portal
                                std::thread::sleep(std::time::Duration::from_secs(2));
                                if let Ok(connectivity) = rt.block_on(async { nm_inst.check_connectivity().await }) {
                                    if connectivity == 2 {
                                        let _ = tx.send_blocking(AppEvent::CaptivePortal(ssid));
                                    }
                                }
                            }
                            Err(e) => { let _ = tx.send_blocking(AppEvent::Error(format!("Connect failed: {}", e))); }
                        }
                    }
                });
            }
        }
    });
    
    // WiFi Details
    let nm_details = nm.clone();
    let rt_details = rt.clone();
    let tx_details = tx.clone();
    win.network_list().set_on_details(move |ssid: String| {
        let nm = nm_details.clone();
        let rt = rt_details.clone();
        let tx = tx_details.clone();
        std::thread::spawn(move || {
            let nm_guard = nm.lock().unwrap();
            if let Some(ref nm_inst) = *nm_guard {
                match rt.block_on(async { nm_inst.get_network_details(&ssid).await }) {
                    Ok(details) => {
                        let _ = tx.send_blocking(AppEvent::NetworkDetailsResult(details));
                    }
                    Err(e) => {
                        let _ = tx.send_blocking(AppEvent::Error(format!("Failed to get network details: {}", e)));
                    }
                }
            }
        });
    });
    
    // Bluetooth Scan
    let bt_scan = bt.clone();
    let rt_bt = rt.clone();
    let tx_bt = tx.clone();
    let dev_list = win.device_list().clone();
    win.device_list().scan_button().connect_clicked(move |_| {
        dev_list.show_scanning();
        let bt = bt_scan.clone();
        let rt = rt_bt.clone();
        let tx = tx_bt.clone();
        std::thread::spawn(move || {
            let bt_guard = bt.lock().unwrap();
            if let Some(ref bt_inst) = *bt_guard {
                let _ = rt.block_on(async { bt_inst.start_discovery().await });
                std::thread::sleep(std::time::Duration::from_secs(5));
                let _ = rt.block_on(async { bt_inst.stop_discovery().await });
                if let Ok(devices) = rt.block_on(async { bt_inst.get_devices().await }) {
                    let _ = tx.send_blocking(AppEvent::BtScanResult(devices));
                }
            }
        });
    });
    
    // Bluetooth Action
    let bt_act = bt.clone();
    let rt_act = rt.clone();
    let tx_act = tx.clone();
    win.device_list().set_on_action(move |path: String, action: DeviceAction| {
        let bt = bt_act.clone();
        let rt = rt_act.clone();
        let tx = tx_act.clone();
        let _ = tx.send_blocking(AppEvent::BtActionStarted(path.clone(), action.clone()));
        std::thread::spawn(move || {
            let bt_guard = bt.lock().unwrap();
            if let Some(ref bt_inst) = *bt_guard {
                let res = match action {
                    DeviceAction::Connect => rt.block_on(async { bt_inst.connect_device(&path).await }),
                    DeviceAction::Disconnect => rt.block_on(async { bt_inst.disconnect_device(&path).await }),
                    DeviceAction::Pair => rt.block_on(async { bt_inst.pair_device(&path).await }),
                    DeviceAction::Forget => rt.block_on(async { bt_inst.forget_device(&path).await }),
                };
                match res {
                    Ok(()) => {
                        let _ = tx.send_blocking(AppEvent::BtActionComplete);
                        std::thread::sleep(std::time::Duration::from_millis(500));
                        if let Ok(devices) = rt.block_on(async { bt_inst.get_devices().await }) {
                            let _ = tx.send_blocking(AppEvent::BtScanResult(devices));
                        }
                    }
                    Err(e) => {
                        let _ = tx.send_blocking(AppEvent::BtActionComplete);
                        let _ = tx.send_blocking(AppEvent::Error(format!("Bluetooth action failed: {}", e)));
                        // Refresh device list to restore correct state
                        if let Ok(devices) = rt.block_on(async { bt_inst.get_devices().await }) {
                            let _ = tx.send_blocking(AppEvent::BtScanResult(devices));
                        }
                    }
                }
            }
        });
    });
    
    // Power Toggle
    let nm_pwr = nm.clone();
    let bt_pwr = bt.clone();
    let rt_pwr = rt.clone();
    let stack_pwr = win.stack().clone();
    let is_programmatic = win.header().is_programmatic_update();
    let power_init_complete = Arc::new(Mutex::new(false));
    let power_init_wifi = power_init_complete.clone();
    
    win.header().power_switch().connect_state_notify(move |switch| {
        // Skip if this is a programmatic update (not user action)
        if *is_programmatic.borrow() {
            return;
        }
        
        if !*power_init_complete.lock().unwrap() {
            return;
        }
        
        let enabled = switch.is_active();
        let is_wifi = stack_pwr.visible_child_name() == Some("wifi".into());
        let nm = nm_pwr.clone();
        let bt = bt_pwr.clone();
        let rt = rt_pwr.clone();
        std::thread::spawn(move || {
            if is_wifi {
                let nm_guard = nm.lock().unwrap();
                if let Some(ref nm_inst) = *nm_guard {
                    let _ = rt.block_on(async { nm_inst.set_wifi_enabled(enabled).await });
                }
            } else {
                let bt_guard = bt.lock().unwrap();
                if let Some(ref bt_inst) = *bt_guard {
                    let _ = rt.block_on(async { bt_inst.set_powered(enabled).await });
                }
            }
        });
    });
    
    // Mark power state initialization complete after a delay
    glib::timeout_add_local(std::time::Duration::from_millis(500), move || {
        *power_init_wifi.lock().unwrap() = true;
        glib::ControlFlow::Break
    });
}

fn setup_periodic_refresh(
    _win: OrbitWindow,
    nm: Arc<Mutex<Option<NetworkManager>>>,
    bt: Arc<Mutex<Option<BluetoothManager>>>,
    rt: Arc<tokio::runtime::Runtime>,
    tx: async_channel::Sender<AppEvent>,
    is_visible: Rc<RefCell<bool>>,
) {
    glib::spawn_future_local(async move {
        loop {
            let visible = *is_visible.borrow();
            let delay = if visible { 10 } else { 30 };
            glib::timeout_future(std::time::Duration::from_secs(delay)).await;
            
            let nm = nm.clone();
            let bt = bt.clone();
            let rt = rt.clone();
            let tx = tx.clone();
            
            std::thread::spawn(move || {
                let nm_guard = nm.lock().unwrap();
                if let Some(ref nm_inst) = *nm_guard {
                    if visible {
                        if let Ok(aps) = rt.block_on(async { nm_inst.get_access_points().await }) {
                            let _ = tx.send_blocking(AppEvent::WifiScanResult(aps));
                        }
                    }
                    if let Ok(enabled) = rt.block_on(async { nm_inst.is_wifi_enabled().await }) {
                        let _ = tx.send_blocking(AppEvent::WifiPowerState(enabled));
                    }
                }
                
                let bt_guard = bt.lock().unwrap();
                if let Some(ref bt_inst) = *bt_guard {
                    if visible {
                        if let Ok(devices) = rt.block_on(async { bt_inst.get_devices().await }) {
                            let _ = tx.send_blocking(AppEvent::BtScanResult(devices));
                        }
                    }
                    if let Ok(powered) = rt.block_on(async { bt_inst.is_powered().await }) {
                        let _ = tx.send_blocking(AppEvent::BtPowerState(powered));
                    }
                }
            });
        }
    });
}
