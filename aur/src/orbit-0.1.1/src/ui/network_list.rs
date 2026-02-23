use gtk4::prelude::*;
use gtk4::{self as gtk, Orientation};
use std::cell::RefCell;
use std::rc::Rc;
use crate::theme::Theme;
use crate::dbus::network_manager::{AccessPoint, SecurityType};

#[derive(Clone)]
pub struct NetworkList {
    container: gtk::Box,
    list_box: gtk::Box,
    scan_button: gtk::Button,
    theme: Rc<RefCell<Theme>>,
    networks: Rc<RefCell<Vec<AccessPoint>>>,
    on_connect: Rc<RefCell<Option<Rc<dyn Fn(AccessPoint)>>>>,
    on_details: Rc<RefCell<Option<Rc<dyn Fn(String)>>>>,
}

impl NetworkList {
    pub fn new(theme: Rc<RefCell<Theme>>) -> Self {
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
            .build();
        
        let scan_button = gtk::Button::builder()
            .label(" Scan for Networks")
            .css_classes(["orbit-button", "primary"])
            .hexpand(true)
            .build();
        
        footer.append(&scan_button);
        container.append(&footer);
        
        let list = Self {
            container,
            list_box,
            scan_button,
            theme,
            networks: Rc::new(RefCell::new(Vec::new())),
            on_connect: Rc::new(RefCell::new(None)),
            on_details: Rc::new(RefCell::new(None)),
        };
        
        list.show_placeholder();
        list
    }
    
    fn show_placeholder(&self) {
        let placeholder = gtk::Label::builder()
            .label("Click 'Scan' to find networks")
            .css_classes(["orbit-placeholder"])
            .build();
        self.list_box.append(&placeholder);
    }
    
    fn get_signal_icon_name(strength: u8) -> &'static str {
        match strength {
            0..=24 => "network-wireless-signal-weak-symbolic",
            25..=49 => "network-wireless-signal-ok-symbolic",
            50..=74 => "network-wireless-signal-good-symbolic",
            _ => "network-wireless-signal-excellent-symbolic",
        }
    }
    
    pub fn set_networks(&self, networks: Vec<AccessPoint>) {
        *self.networks.borrow_mut() = networks.clone();
        
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
        let css_classes = if network.is_connected {
            vec!["orbit-network-row", "connected"]
        } else {
            vec!["orbit-network-row"]
        };
        
        let row = gtk::Box::builder()
            .orientation(Orientation::Horizontal)
            .spacing(12)
            .css_classes(css_classes)
            .build();
        
        if network.is_connected {
            let icon_container = gtk::Box::builder()
                .css_classes(["orbit-icon-container"])
                .halign(gtk::Align::Center)
                .valign(gtk::Align::Center)
                .build();
            
            let signal_icon = gtk::Image::builder()
                .icon_name("network-wireless-symbolic")
                .pixel_size(20)
                .css_classes(["orbit-icon-accent"])
                .build();
            icon_container.append(&signal_icon);
            row.append(&icon_container);
        } else {
            let signal_icon = gtk::Image::builder()
                .icon_name(Self::get_signal_icon_name(network.signal_strength))
                .css_classes(["orbit-signal-icon"])
                .pixel_size(20)
                .build();
            row.append(&signal_icon);
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
        
        if network.security != SecurityType::None && !network.is_connected {
            let lock_icon = gtk::Image::builder()
                .icon_name("system-lock-screen-symbolic")
                .pixel_size(14)
                .css_classes(["orbit-signal-icon"])
                .build();
            actions_box.append(&lock_icon);
        }
        
        let action_label = if network.is_connected { "Disconnect" } else { "Connect" };
        let action_btn = gtk::Button::builder()
            .label(action_label)
            .css_classes(if network.is_connected { 
                vec!["orbit-button"] 
            } else { 
                vec!["orbit-button", "primary"] 
            })
            .build();
        
        let network_clone = network.clone();
        let on_connect = self.on_connect.clone();
        action_btn.connect_clicked(move |_| {
            if let Some(callback) = on_connect.borrow().as_ref() {
                callback(network_clone.clone());
            }
        });
        
        actions_box.append(&action_btn);
        
        if network.is_connected {
            let details_btn = gtk::Button::builder()
                .label("Details")
                .css_classes(["orbit-button"])
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
        
        row.append(&actions_box);
        row
    }
    
    pub fn widget(&self) -> &gtk::Box {
        &self.container
    }
    
    pub fn scan_button(&self) -> &gtk::Button {
        &self.scan_button
    }
    
    pub fn show_scanning(&self) {
        while let Some(child) = self.list_box.first_child() {
            self.list_box.remove(&child);
        }
        
        let scanning = gtk::Label::builder()
            .label("Scanning for networks...")
            .css_classes(["orbit-placeholder"])
            .build();
        self.list_box.append(&scanning);
    }
    
    pub fn set_on_connect<F: Fn(AccessPoint) + 'static>(&self, callback: F) {
        *self.on_connect.borrow_mut() = Some(Rc::new(callback));
    }
    
    pub fn set_on_details<F: Fn(String) + 'static>(&self, callback: F) {
        *self.on_details.borrow_mut() = Some(Rc::new(callback));
    }
}
