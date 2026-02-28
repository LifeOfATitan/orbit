use gtk4::prelude::*;
use gtk4::{self as gtk, Orientation};
use std::cell::RefCell;
use std::rc::Rc;
use std::collections::HashMap;
use crate::dbus::network_manager::{AccessPoint, SecurityType};

#[derive(Clone)]
pub struct NetworkList {
    container: gtk::Box,
    list_box: gtk::Box,
    scan_button: gtk::Button,
    networks: Rc<RefCell<Vec<AccessPoint>>>,
    row_actions: Rc<RefCell<HashMap<String, gtk::Box>>>,
    on_connect: Rc<RefCell<Option<Rc<dyn Fn(AccessPoint)>>>>,
    on_connect_hidden: Rc<RefCell<Option<Rc<dyn Fn()>>>>,
    on_details: Rc<RefCell<Option<Rc<dyn Fn(String)>>>>,
    connecting_ssid: Rc<RefCell<Option<String>>>,
    disconnecting_ssid: Rc<RefCell<Option<String>>>,
}

impl NetworkList {
    pub fn new() -> Self {
        let container = gtk::Box::builder()
            .orientation(Orientation::Vertical)
            .vexpand(true)
            .hexpand(true)
            .build();
        
        let scrolled = gtk::ScrolledWindow::builder()
            .vexpand(true)
            .hexpand(true)
            .hscrollbar_policy(gtk::PolicyType::Never)
            .min_content_height(280)
            .css_classes(["orbit-scrolled"])
            .build();
        
        let list_box = gtk::Box::builder()
            .orientation(Orientation::Vertical)
            .css_classes(["orbit-list"])
            .build();
        
        scrolled.set_child(Some(&list_box));
        container.append(&scrolled);
        
        let footer = gtk::Box::builder()
            .css_classes(["orbit-footer"])
            .margin_top(8)
            .spacing(8)
            .homogeneous(true)
            .build();
        
        let scan_button = gtk::Button::builder()
            .label(" Scan for Networks")
            .css_classes(["orbit-button", "primary", "flat"])
            .hexpand(true)
            .build();

        let hidden_button = gtk::Button::builder()
            .label(" Hidden Network")
            .css_classes(["orbit-button", "flat"])
            .hexpand(true)
            .build();
        
        footer.append(&scan_button);
        footer.append(&hidden_button);
        container.append(&footer);
        
        let list = Self {
            container,
            list_box,
            scan_button,
            networks: Rc::new(RefCell::new(Vec::new())),
            row_actions: Rc::new(RefCell::new(HashMap::new())),
            on_connect: Rc::new(RefCell::new(None)),
            on_connect_hidden: Rc::new(RefCell::new(None)),
            on_details: Rc::new(RefCell::new(None)),
            connecting_ssid: Rc::new(RefCell::new(None)),
            disconnecting_ssid: Rc::new(RefCell::new(None)),
        };

        let on_connect_hidden_cb = list.on_connect_hidden.clone();
        hidden_button.connect_clicked(move |_| {
            if let Some(cb) = on_connect_hidden_cb.borrow().as_ref() {
                cb();
            }
        });
        
        list.show_loading();
        list
    }
    
    fn show_loading(&self) {
        let placeholder = gtk::Label::builder()
            .label("Loading networks...")
            .css_classes(["orbit-placeholder"])
            .build();
        self.list_box.append(&placeholder);
    }
    
    fn show_placeholder(&self) {
        let placeholder = gtk::Label::builder()
            .label("Click 'Scan' to find networks")
            .css_classes(["orbit-placeholder"])
            .build();
        self.list_box.append(&placeholder);
    }
    
    fn signal_bar_count(strength: u8) -> u8 {
        match strength {
            0..=24 => 1,
            25..=49 => 2,
            50..=74 => 3,
            _ => 4,
        }
    }
    
    fn build_signal_bars(strength: u8, is_connected: bool) -> gtk::Box {
        let active_bars = Self::signal_bar_count(strength);
        let heights = [4, 8, 12, 16];
        
        let container = gtk::Box::builder()
            .orientation(Orientation::Horizontal)
            .spacing(2)
            .valign(gtk::Align::End)
            .halign(gtk::Align::Center)
            .build();
        
        for (i, &h) in heights.iter().enumerate() {
            let bar_num = (i + 1) as u8;
            let active = bar_num <= active_bars;
            
            let bar = gtk::Box::builder()
                .width_request(3)
                .height_request(h)
                .valign(gtk::Align::End)
                .build();
            
            if active {
                if is_connected {
                    bar.add_css_class("orbit-signal-bar-active-accent");
                } else {
                    bar.add_css_class("orbit-signal-bar-active");
                }
            } else {
                bar.add_css_class("orbit-signal-bar-inactive");
            }
            
            container.append(&bar);
        }
        
        container
    }
    
    pub fn set_connecting_ssid(&self, ssid: Option<String>) {
        let old_ssid = self.connecting_ssid.borrow().clone();
        *self.connecting_ssid.borrow_mut() = ssid.clone();
        
        if let Some(ref s) = ssid {
            self.update_single_row_actions(s);
        }
        if let Some(ref s) = old_ssid {
            self.update_single_row_actions(s);
        }
    }
    
    pub fn set_disconnecting_ssid(&self, ssid: Option<String>) {
        let old_ssid = self.disconnecting_ssid.borrow().clone();
        *self.disconnecting_ssid.borrow_mut() = ssid.clone();
        
        if let Some(ref s) = ssid {
            self.update_single_row_actions(s);
        }
        if let Some(ref s) = old_ssid {
            self.update_single_row_actions(s);
        }
    }
    
    fn update_single_row_actions(&self, ssid: &str) {
        let networks = self.networks.borrow();
        if let Some(network) = networks.iter().find(|n| n.ssid == ssid) {
            let actions_map = self.row_actions.borrow();
            if let Some(actions_box) = actions_map.get(ssid) {
                while let Some(child) = actions_box.first_child() {
                    actions_box.remove(&child);
                }
                self.build_actions_box_content(actions_box, network);
            }
        }
    }

    pub fn set_networks(&self, networks: Vec<AccessPoint>) {
        *self.networks.borrow_mut() = networks.clone();
        *self.connecting_ssid.borrow_mut() = None;
        *self.disconnecting_ssid.borrow_mut() = None;
        self.render_networks(&networks);
    }
    
    fn render_networks(&self, networks: &[AccessPoint]) {
        self.row_actions.borrow_mut().clear();

        while let Some(child) = self.list_box.first_child() {
            self.list_box.remove(&child);
        }
        
        if networks.is_empty() {
            self.show_placeholder();
            return;
        }
        
        let connected_networks: Vec<&AccessPoint> = networks.iter().filter(|n| n.is_connected).collect();
        let available_networks: Vec<&AccessPoint> = networks.iter().filter(|n| !n.is_connected).collect();
        
        if !connected_networks.is_empty() {
            let section_header = gtk::Label::builder()
                .label("ACTIVE CONNECTION")
                .css_classes(["orbit-section-header"])
                .halign(gtk::Align::Start)
                .build();
            self.list_box.append(&section_header);
            
            for network in connected_networks {
                let row = self.create_network_row(network);
                self.list_box.append(&row);
            }
        }
        
        if !available_networks.is_empty() {
            let section_header = gtk::Label::builder()
                .label("AVAILABLE NETWORKS")
                .css_classes(["orbit-section-header"])
                .halign(gtk::Align::Start)
                .build();
            self.list_box.append(&section_header);
            
            for network in available_networks {
                let row = self.create_network_row(network);
                self.list_box.append(&row);
            }
        }
    }
    
    fn create_network_row(&self, network: &AccessPoint) -> gtk::Box {
        let row = gtk::Box::builder()
            .orientation(Orientation::Horizontal)
            .spacing(12)
            .css_classes(["orbit-network-row"])
            .focusable(true)
            .build();
        
        let row_focus = row.clone();
        let focus_in = gtk::EventControllerFocus::new();
        focus_in.connect_enter(move |_| {
            row_focus.add_css_class("focused");
        });
        let row_unfocus = row.clone();
        let focus_out = gtk::EventControllerFocus::new();
        focus_out.connect_leave(move |_| {
            row_unfocus.remove_css_class("focused");
        });
        row.add_controller(focus_in);
        row.add_controller(focus_out);

        if network.is_connected {
            let icon_container = gtk::Box::builder()
                .css_classes(["orbit-icon-container"])
                .halign(gtk::Align::Center)
                .valign(gtk::Align::Center)
                .build();
            
            let signal_bars = Self::build_signal_bars(network.signal_strength, true);
            icon_container.append(&signal_bars);
            row.append(&icon_container);
        } else {
            let signal_bars = Self::build_signal_bars(network.signal_strength, false);
            signal_bars.set_valign(gtk::Align::Center);
            signal_bars.add_css_class("orbit-signal-bars-pad");
            row.append(&signal_bars);
        }
        
        let info_box = gtk::Box::builder()
            .orientation(Orientation::Vertical)
            .spacing(2)
            .hexpand(true)
            .valign(gtk::Align::Center)
            .build();
        
        let ssid = gtk::Label::builder()
            .label(&network.ssid)
            .css_classes(["orbit-ssid"])
            .halign(gtk::Align::Start)
            .build();
        info_box.append(&ssid);
        
        let status_text = if network.is_connected {
            format!("Connected · {}%", network.signal_strength)
        } else {
            let security = if network.security != SecurityType::None { "Secure" } else { "Open" };
            format!("{}% Signal · {}", network.signal_strength, security)
        };
        
        let status = gtk::Label::builder()
            .label(&status_text)
            .css_classes(["orbit-status"])
            .halign(gtk::Align::Start)
            .build();
        info_box.append(&status);
        
        row.append(&info_box);
        
        let actions_box = gtk::Box::builder()
            .orientation(Orientation::Horizontal)
            .spacing(8)
            .build();
        
        self.build_actions_box_content(&actions_box, network);
        
        self.row_actions.borrow_mut().insert(network.ssid.clone(), actions_box.clone());

        row.append(&actions_box);
        row
    }

    fn build_actions_box_content(&self, actions_box: &gtk::Box, network: &AccessPoint) {
        if network.security != SecurityType::None && !network.is_connected {
            let lock_icon = gtk::Image::builder()
                .icon_name("system-lock-screen-symbolic")
                .pixel_size(14)
                .css_classes(["orbit-signal-icon"])
                .build();
            actions_box.append(&lock_icon);
        }
        
        let is_connecting = self.connecting_ssid.borrow().as_deref() == Some(&network.ssid);
        let is_disconnecting = self.disconnecting_ssid.borrow().as_deref() == Some(&network.ssid);
        let any_connecting = self.connecting_ssid.borrow().is_some();
        let any_disconnecting = self.disconnecting_ssid.borrow().is_some();
        
        if is_connecting || is_disconnecting {
            let working_box = gtk::Box::builder()
                .orientation(Orientation::Horizontal)
                .spacing(8)
                .css_classes(["orbit-working-indicator"])
                .build();
            
            let spinner = gtk::Spinner::builder()
                .spinning(true)
                .build();
            spinner.start();
            
            let label = gtk::Label::builder()
                .label(if is_connecting { "Connecting..." } else { "Disconnecting..." })
                .css_classes(["orbit-status"])
                .build();
            
            working_box.append(&spinner);
            working_box.append(&label);
            actions_box.append(&working_box);
        } else {
            let action_label = if network.is_connected {
                "Disconnect"
            } else {
                "Connect"
            };
            
            let btn_classes = if network.is_connected { 
                vec!["orbit-button", "flat"] 
            } else { 
                vec!["orbit-button", "primary", "flat"] 
            };
            
            let action_btn = gtk::Button::builder()
                .label(action_label)
                .css_classes(btn_classes)
                .sensitive(!(any_connecting && !network.is_connected) && !(any_disconnecting && network.is_connected))
                .build();
            
            let network_clone = network.clone();
            let on_connect = self.on_connect.clone();
            action_btn.connect_clicked(move |_| {
                if let Some(callback) = on_connect.borrow().as_ref() {
                    callback(network_clone.clone());
                }
            });
            
            actions_box.append(&action_btn);
        }
        
        if network.is_connected && !is_disconnecting {
            let details_btn = gtk::Button::builder()
                .label("Details")
                .css_classes(["orbit-button", "flat"])
                .build();
            
            let ssid = network.ssid.clone();
            let on_details = self.on_details.clone();
            details_btn.connect_clicked(move |_| {
                if let Some(callback) = on_details.borrow().as_ref() {
                    callback(ssid.clone());
                }
            });
            
            actions_box.append(&details_btn);
        }
    }
    
    pub fn widget(&self) -> &gtk::Box {
        &self.container
    }
    
    pub fn scan_button(&self) -> &gtk::Button {
        &self.scan_button
    }
    
    pub fn set_on_connect<F: Fn(AccessPoint) + 'static>(&self, callback: F) {
        *self.on_connect.borrow_mut() = Some(Rc::new(callback));
    }
    
    pub fn set_on_connect_hidden<F: Fn() + 'static>(&self, callback: F) {
        *self.on_connect_hidden.borrow_mut() = Some(Rc::new(callback));
    }
    
    pub fn set_on_details<F: Fn(String) + 'static>(&self, callback: F) {
        *self.on_details.borrow_mut() = Some(Rc::new(callback));
    }
}
