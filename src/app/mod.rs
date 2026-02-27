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
    DisconnectStarted(String),
    BtActionStarted(String, DeviceAction),
    BtActionComplete,
    Error(String),
    Notify(String),
    CaptivePortal(String),
    DaemonCommand(DaemonCommand),
    DaemonStarted(DaemonServer),
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
            let app_quit = app.clone();
            glib::unix_signal_add_local(15, move || {
                app_quit.quit();
                glib::ControlFlow::Break
            });
            let app_quit_int = app.clone();
            glib::unix_signal_add_local(2, move || {
                app_quit_int.quit();
                glib::ControlFlow::Break
            });

            let config = config.clone();
            let win_theme = win_theme.clone();
            
            let rt = Arc::new(tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime"));
            let win = OrbitWindow::new(app, config, win_theme.clone());
            
            let nm: Arc<Mutex<Option<NetworkManager>>> = Arc::new(Mutex::new(None));
            let bt: Arc<Mutex<Option<BluetoothManager>>> = Arc::new(Mutex::new(None));
            
            let (tx, rx) = async_channel::unbounded::<AppEvent>();
            
            // Initialization thread
            {
                let rt_init = rt.clone();
                let nm_arc = nm.clone();
                let bt_arc = bt.clone();
                let tx_init = tx.clone();
                
                std::thread::spawn(move || {
                    // 1. Start Daemon Server IMMEDIATELY
                    if is_daemon {
                        match rt_init.block_on(async { DaemonServer::new().await }) {
                            Ok(server) => {
                                let _ = tx_init.send_blocking(AppEvent::DaemonStarted(server));
                            }
                            Err(e) => {
                                log::error!("Failed to start daemon server: {}", e);
                                eprintln!("Error: {}", e);
                                std::process::exit(1);
                            }
                        }
                    }

                    // 2. Initialize DBus Managers
                    let mut nm_inst = None;
                    for i in 0..5 {
                        if let Ok(inst) = rt_init.block_on(async { NetworkManager::new().await }) {
                            nm_inst = Some(inst);
                            break;
                        }
                        if i < 4 {
                            std::thread::sleep(std::time::Duration::from_secs(1));
                        }
                    }

                    let mut bt_inst = None;
                    for i in 0..5 {
                        if let Ok(inst) = rt_init.block_on(async { BluetoothManager::new().await }) {
                            bt_inst = Some(inst);
                            break;
                        }
                        if i < 4 {
                            std::thread::sleep(std::time::Duration::from_secs(1));
                        }
                    }
                    
                    if let Some(ref nm) = nm_inst {
                        if let Ok(enabled) = rt_init.block_on(async { nm.is_wifi_enabled().await }) {
                            let _ = tx_init.send_blocking(AppEvent::WifiPowerState(enabled));
                            
                            if enabled {
                                log::info!("Not connected yet, waiting for NetworkManager...");
                                let mut attempts = 0;
                                let mut connected_ssid = None;
                                
                                while attempts < 15 {
                                    if let Some(ssid) = rt_init.block_on(async { nm.get_active_ssid().await }) {
                                        log::info!("Connected to {} after {}s", ssid, attempts);
                                        let _ = tx_init.send_blocking(AppEvent::Notify(format!("Connected to {}", ssid)));
                                        connected_ssid = Some(ssid);
                                        break;
                                    }
                                    
                                    let state = rt_init.block_on(async { nm.get_wifi_device_state().await }).unwrap_or(0);
                                    if state < 30 || state > 100 {
                                        if attempts >= 5 {
                                            log::info!("NetworkManager is idle (state {}), stopping wait", state);
                                            break;
                                        }
                                    } else {
                                        log::info!("NetworkManager is busy (state {}), waiting...", state);
                                    }

                                    std::thread::sleep(std::time::Duration::from_secs(1));
                                    attempts += 1;
                                }

                                if connected_ssid.is_none() {
                                    log::info!("Autoconnect timed out, triggering scan");
                                    let _ = rt_init.block_on(async { nm.scan().await });
                                }
                                
                                if let Some(ssid) = connected_ssid {
                                    std::thread::sleep(std::time::Duration::from_secs(2));
                                    if let Ok(connectivity) = rt_init.block_on(async { nm.check_connectivity().await }) {
                                        if connectivity == 2 {
                                            let _ = tx_init.send_blocking(AppEvent::CaptivePortal(ssid));
                                        }
                                    }
                                }
                            }
                            
                            if let Ok(saved) = rt_init.block_on(async { nm.get_saved_networks().await }) {
                                let _ = tx_init.send_blocking(AppEvent::SavedNetworksResult(saved));
                            }
                        }
                    }
                    
                    if let Some(ref bt) = bt_inst {
                        if let Ok(powered) = rt_init.block_on(async { bt.is_powered().await }) {
                            let _ = tx_init.send_blocking(AppEvent::BtPowerState(powered));
                        }
                        if let Ok(devices) = rt_init.block_on(async { bt.get_devices().await }) {
                            let _ = tx_init.send_blocking(AppEvent::BtScanResult(devices));
                        }
                    }
                    
                    let mut nm_guard = nm_arc.lock().unwrap();
                    *nm_guard = nm_inst;
                    let mut bt_guard = bt_arc.lock().unwrap();
                    *bt_guard = bt_inst;
                });
            }
            
            let is_visible = Rc::new(RefCell::new(!is_daemon));
            
            let is_visible_sync = is_visible.clone();
            win.window().connect_notify_local(Some("visible"), move |window, _| {
                *is_visible_sync.borrow_mut() = window.is_visible();
            });
            
            if !is_daemon {
                win.show();
            }
            
            setup_events_receiver(win.clone(), rx.clone(), is_visible.clone(), nm.clone(), bt.clone(), rt.clone(), tx.clone(), win_theme.clone());
            setup_ui_callbacks(win.clone(), nm.clone(), bt.clone(), rt.clone(), tx.clone());
            setup_periodic_refresh(win.clone(), nm, bt, rt.clone(), tx.clone(), is_visible.clone());
        });
        
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
                    if let Some(tab) = win.stack().visible_child_name() {
                        let tab_str = tab.as_str();
                        if tab_str == "wifi" || tab_str == "saved" {
                            win.header().set_power_state(enabled);
                        }
                    }
                }
                AppEvent::BtPowerState(enabled) => {
                    if let Some(tab) = win.stack().visible_child_name() {
                        let tab_str = tab.as_str();
                        if tab_str == "bluetooth" {
                            win.header().set_power_state(enabled);
                        }
                    }
                }
                AppEvent::Error(msg) => {
                    win.network_list().set_connecting_ssid(None);
                    win.network_list().set_disconnecting_ssid(None);
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
                AppEvent::DisconnectStarted(ssid) => {
                    win.network_list().set_disconnecting_ssid(Some(ssid));
                }
                AppEvent::ConnectSuccess => {
                    win.network_list().set_connecting_ssid(None);
                    win.network_list().set_disconnecting_ssid(None);
                    win.hide_password_dialog();
                }
                AppEvent::ConnectHidden(ssid, password) => {
                    let nm_ref = nm.clone();
                    let rt_ref = rt.clone();
                    let tx_ref = tx.clone();
                    
                    std::thread::spawn(move || {
                        let nm_guard = nm_ref.lock().unwrap();
                        if let Some(ref nm_inst) = *nm_guard {
                            let rt_inner = rt_ref.clone();
                            match rt_inner.block_on(async { nm_inst.get_wireless_devices().await }) {
                                Ok(devices) => {
                                    if let Some(device_path) = devices.get(0) {
                                        let pwd = if password.is_empty() { None } else { Some(password.as_str()) };
                                        let rt_conn = rt_ref.clone();
                                        match rt_conn.block_on(async { nm_inst.connect_hidden(&ssid, pwd, device_path).await }) {
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
                AppEvent::DaemonCommand(cmd) => {
                    match cmd {
                        DaemonCommand::Show => {
                            win.show();
                            *is_visible.borrow_mut() = true;
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
                AppEvent::DaemonStarted(server) => {
                    let tx_cmd = tx.clone();
                    server.run(move |cmd| {
                        let _ = tx_cmd.send_blocking(AppEvent::DaemonCommand(cmd));
                    });
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
    let nm_scan = nm.clone();
    let rt_scan = rt.clone();
    let tx_scan = tx.clone();
    win.network_list().scan_button().connect_clicked(move |_| {
        let nm = nm_scan.clone();
        let rt = rt_scan.clone();
        let tx = tx_scan.clone();
        std::thread::spawn(move || {
            let nm_guard = nm.lock().unwrap();
            if let Some(ref nm_inst) = *nm_guard {
                let _ = rt.block_on(async { nm_inst.scan().await });
                std::thread::sleep(std::time::Duration::from_millis(1500));
                if let Ok(aps) = rt.block_on(async { nm_inst.get_access_points().await }) {
                    let _ = tx.send_blocking(AppEvent::WifiScanResult(aps));
                }
            }
        });
    });

    let nm_auto = nm.clone();
    let rt_auto = rt.clone();
    let tx_auto = tx.clone();
    win.saved_networks_list().set_on_autoconnect_toggle(move |path: String, enabled: bool| {
        let nm = nm_auto.clone();
        let rt = rt_auto.clone();
        let tx = tx_auto.clone();
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
    
    let nm_conn = nm.clone();
    let rt_conn = rt.clone();
    let tx_conn = tx.clone();
    let win_conn_hidden = win.clone();
    let tx_conn_hidden = tx.clone();
    win.network_list().set_on_connect_hidden(move || {
        let tx = tx_conn_hidden.clone();
        win_conn_hidden.show_hidden_dialog(move |data| {
            if let Some((ssid, password)) = data {
                let _ = tx.send_blocking(AppEvent::ConnectHidden(ssid, password));
            }
        });
    });

    let win_connect = win.clone();
    win.network_list().set_on_connect(move |ap: AccessPoint| {
        let nm = nm_conn.clone();
        let rt = rt_conn.clone();
        let tx = tx_conn.clone();
        let ap_path = ap.device_path.clone();
        let ssid = ap.ssid.clone();
        
        if ap.is_connected {
            let ap_path_inner = ap.path.clone();
            let ssid_inner = ap.ssid.clone();
            let _ = tx.send_blocking(AppEvent::DisconnectStarted(ssid_inner.clone()));
            let nm_val = nm.clone();
            let rt_val = rt.clone();
            let tx_val = tx.clone();
            std::thread::spawn(move || {
                let nm_guard = nm_val.lock().unwrap();
                if let Some(ref nm_inst) = *nm_guard {
                    let _ = rt_val.block_on(async { nm_inst.disconnect_ap(&ssid_inner, &ap_path_inner).await });
                    // Artificial delay for visual feedback (1s)
                    std::thread::sleep(std::time::Duration::from_millis(1000));
                    let _ = tx_val.send_blocking(AppEvent::ConnectSuccess);
                    std::thread::sleep(std::time::Duration::from_millis(500));
                    if let Ok(aps) = rt_val.block_on(async { nm_inst.get_access_points().await }) {
                        let _ = tx_val.send_blocking(AppEvent::WifiScanResult(aps));
                    }
                }
            });
        } else {
            let win_p = win_connect.clone();
            let nm_check = nm.clone();
            let rt_check = rt.clone();
            let ssid_check = ssid.clone();
            
            let has_saved = {
                let nm_guard = nm_check.lock().unwrap();
                if let Some(ref nm_inst) = *nm_guard {
                    rt_check.block_on(async { nm_inst.has_saved_connection(&ssid_check).await })
                } else {
                    false
                }
            };

            if ap.security == SecurityType::None || has_saved {
                let _ = tx.send_blocking(AppEvent::ConnectStarted(ssid.clone()));
                let nm_val = nm.clone();
                let rt_val = rt.clone();
                let tx_val = tx.clone();
                let ssid_val = ssid.clone();
                let ap_path_val = ap_path.clone();
                std::thread::spawn(move || {
                    log::info!("UI: Connect clicked for network: '{}' (Has saved: {})", ssid_val, has_saved);
                    let nm_guard = nm_val.lock().unwrap();
                    if let Some(ref nm_inst) = *nm_guard {
                        match rt_val.block_on(async { nm_inst.connect_to_network(&ssid_val, None, &ap_path_val).await }) {
                            Ok(()) => {
                                // Artificial delay for visual feedback (1s)
                                std::thread::sleep(std::time::Duration::from_millis(1000));
                                let _ = tx_val.send_blocking(AppEvent::ConnectSuccess);
                                let _ = tx_val.send_blocking(AppEvent::Notify(format!("Connected to {}", ssid_val)));
                                if let Ok(aps) = rt_val.block_on(async { nm_inst.get_access_points().await }) {
                                    let _ = tx_val.send_blocking(AppEvent::WifiScanResult(aps));
                                }
                            }
                            Err(e) => { 
                                log::error!("UI: Connect failed for '{}': {}", ssid_val, e);
                                let _ = tx_val.send_blocking(AppEvent::Error(format!("Connect failed: {}", e))); 
                            }
                        }
                    }
                });
            } else {
                let ssid_val = ssid.clone();
                let nm_val = nm.clone();
                let rt_val = rt.clone();
                let tx_val = tx.clone();
                let ap_path_val = ap_path.clone();
                win_p.show_password_dialog(&ssid, move |password| {
                    if let Some(pwd) = password {
                        let nm_inner = nm_val.clone();
                        let rt_inner = rt_val.clone();
                        let tx_inner = tx_val.clone();
                        let ssid_inner = ssid_val.clone();
                        let ap_path_inner = ap_path_val.clone();

                        let _ = tx_inner.send_blocking(AppEvent::ConnectStarted(ssid_inner.clone()));
                        std::thread::spawn(move || {
                            log::info!("UI: Connect clicked (with password) for: '{}'", ssid_inner);
                            let nm_guard = nm_inner.lock().unwrap();
                            if let Some(ref nm_inst) = *nm_guard {
                                match rt_inner.block_on(async { nm_inst.connect_to_network(&ssid_inner, Some(&pwd), &ap_path_inner).await }) {
                                    Ok(()) => {
                                        // Artificial delay for visual feedback (1s)
                                        std::thread::sleep(std::time::Duration::from_millis(1000));
                                        let _ = tx_inner.send_blocking(AppEvent::ConnectSuccess);
                                        let _ = tx_inner.send_blocking(AppEvent::Notify(format!("Connected to {}", ssid_inner)));
                                        if let Ok(aps) = rt_inner.block_on(async { nm_inst.get_access_points().await }) {
                                            let _ = tx_inner.send_blocking(AppEvent::WifiScanResult(aps));
                                        }
                                    }
                                    Err(e) => { 
                                        log::error!("UI: Connect failed for '{}': {}", ssid_inner, e);
                                        let _ = tx_inner.send_blocking(AppEvent::Error(format!("Connect failed: {}", e))); 
                                    }
                                }
                            }
                        });
                    }
                });
            }
        }
    });
    
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
                        if let Ok(devices) = rt.block_on(async { bt_inst.get_devices().await }) {
                            let _ = tx.send_blocking(AppEvent::BtScanResult(devices));
                        }
                    }
                }
            }
        });
    });
    
    let nm_pwr = nm.clone();
    let bt_pwr = bt.clone();
    let rt_pwr = rt.clone();
    let tx_pwr = tx.clone();
    let win_pwr = win.clone();
    win.header().power_switch().connect_state_set(move |_, enabled| {
        let nm = nm_pwr.clone();
        let bt = bt_pwr.clone();
        let rt = rt_pwr.clone();
        let tx = tx_pwr.clone();
        let win_inner = win_pwr.clone();
        let current_tab = win_inner.stack().visible_child_name().map(|s| s.to_string());
        
        std::thread::spawn(move || {
            if let Some(tab) = current_tab {
                if tab == "wifi" || tab == "saved" {
                    let nm_guard = nm.lock().unwrap();
                    if let Some(ref nm_inst) = *nm_guard {
                        let _ = rt.block_on(async { nm_inst.set_wifi_enabled(enabled).await });
                        let _ = tx.send_blocking(AppEvent::WifiPowerState(enabled));
                        if enabled {
                            std::thread::sleep(std::time::Duration::from_millis(1500));
                            if let Ok(aps) = rt.block_on(async { nm_inst.get_access_points().await }) {
                                let _ = tx.send_blocking(AppEvent::WifiScanResult(aps));
                            }
                        }
                    }
                } else if tab == "bluetooth" {
                    let bt_guard = bt.lock().unwrap();
                    if let Some(ref bt_inst) = *bt_guard {
                        let _ = rt.block_on(async { bt_inst.set_powered(enabled).await });
                        let _ = tx.send_blocking(AppEvent::BtPowerState(enabled));
                        if enabled {
                            std::thread::sleep(std::time::Duration::from_millis(1500));
                            if let Ok(devices) = rt.block_on(async { bt_inst.get_devices().await }) {
                                let _ = tx.send_blocking(AppEvent::BtScanResult(devices));
                            }
                        }
                    }
                }
            }
        });
        glib::Propagation::Proceed
    });
}

fn setup_periodic_refresh(
    win: OrbitWindow,
    nm: Arc<Mutex<Option<NetworkManager>>>,
    bt: Arc<Mutex<Option<BluetoothManager>>>,
    rt: Arc<tokio::runtime::Runtime>,
    tx: async_channel::Sender<AppEvent>,
    is_visible: Rc<RefCell<bool>>,
) {
    glib::timeout_add_local(std::time::Duration::from_secs(5), move || {
        if !*is_visible.borrow() {
            return glib::ControlFlow::Continue;
        }
        
        let nm = nm.clone();
        let bt = bt.clone();
        let rt = rt.clone();
        let tx = tx.clone();
        let win_refresh = win.clone();
        let current_tab = win_refresh.stack().visible_child_name().map(|s| s.to_string());
        
        std::thread::spawn(move || {
            if let Some(tab) = current_tab {
                if tab == "wifi" {
                    let nm_guard = nm.lock().unwrap();
                    if let Some(ref nm_inst) = *nm_guard {
                        if let Ok(aps) = rt.block_on(async { nm_inst.get_access_points().await }) {
                            let _ = tx.send_blocking(AppEvent::WifiScanResult(aps));
                        }
                    }
                } else if tab == "bluetooth" {
                    let bt_guard = bt.lock().unwrap();
                    if let Some(ref bt_inst) = *bt_guard {
                        if let Ok(devices) = rt.block_on(async { bt_inst.get_devices().await }) {
                            let _ = tx.send_blocking(AppEvent::BtScanResult(devices));
                        }
                    }
                } else if tab == "saved" {
                    let nm_guard = nm.lock().unwrap();
                    if let Some(ref nm_inst) = *nm_guard {
                        if let Ok(saved) = rt.block_on(async { nm_inst.get_saved_networks().await }) {
                            let _ = tx.send_blocking(AppEvent::SavedNetworksResult(saved));
                        }
                    }
                }
            }
        });
        
        glib::ControlFlow::Continue
    });
}
