use gtk4::prelude::*;
use gtk4::{self as gtk, glib, Orientation};
use std::cell::RefCell;
use std::rc::Rc;
use crate::theme::Theme;
use crate::dbus::network_manager::VpnConnection;

#[derive(Clone)]
pub struct VpnList {
    container: gtk::Box,
    list_box: gtk::Box,
    refresh_button: gtk::Button,
    theme: Rc<RefCell<Theme>>,
    networks: Rc<RefCell<Vec<VpnConnection>>>,
}

impl VpnList {
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
        
        let refresh_button = gtk::Button::builder()
            .label(" Refresh VPNs")
            .css_classes(["orbit-button", "primary", "flat"])
            .hexpand(true)
            .build();
        
        footer.append(&refresh_button);
        container.append(&footer);
        
        let list = Self {
            container,
            list_box,
            refresh_button,
            theme,
            networks: Rc::new(RefCell::new(Vec::new())),
        };
        
        list.show_placeholder();
        list
    }
    
    fn show_placeholder(&self) {
        let placeholder = gtk::Label::builder()
            .label("No vpns found")
            .css_classes(["orbit-placeholder"])
            .build();
        self.list_box.append(&placeholder);
    }
    
    pub fn set_networks(&self, networks: Vec<VpnConnection>) {
        *self.networks.borrow_mut() = networks.clone();
        
        while let Some(child) = self.list_box.first_child() {
            self.list_box.remove(&child);
        }
        
        if networks.is_empty() {
            self.show_placeholder();
            return;
        }
        
        let active_networks: Vec<&VpnConnection> = networks.iter().filter(|n| n.is_active).collect();
        let saved_networks: Vec<&VpnConnection> = networks.iter().filter(|n| !n.is_active).collect();
        
        if !active_networks.is_empty() {
            let section_header = gtk::Label::builder()
                .label("CURRENTLY CONNECTED")
                .css_classes(["orbit-section-header"])
                .halign(gtk::Align::Start)
                .build();
            self.list_box.append(&section_header);
            
            for network in active_networks {
                let row = self.create_network_row(network);
                self.list_box.append(&row);
            }
        }
        
        if !saved_networks.is_empty() {
            let section_header = gtk::Label::builder()
                .label("VPN CONNECTIONS")
                .css_classes(["orbit-section-header"])
                .halign(gtk::Align::Start)
                .build();
            self.list_box.append(&section_header);
            
            for network in saved_networks {
                let row = self.create_network_row(network);
                self.list_box.append(&row);
            }
        }
    }
    
    fn create_network_row(&self, network: &VpnConnection) -> gtk::Box {
        let css_classes = if network.is_active {
            vec!["orbit-saved-network-row", "active"]
        } else {
            vec!["orbit-saved-network-row"]
        };
        
        let row = gtk::Box::builder()
            .orientation(Orientation::Horizontal)
            .spacing(12)
            .css_classes(css_classes)
            .build();
        
        if network.is_active {
            let icon_container = gtk::Box::builder()
                .css_classes(["orbit-icon-container"])
                .halign(gtk::Align::Center)
                .valign(gtk::Align::Center)
                .build();
            
            let vpn_icon = gtk::Image::builder()
                .icon_name("network-vpn-symbolic")
                .pixel_size(20)
                .css_classes(["orbit-icon-accent"])
                .build();
            icon_container.append(&vpn_icon);
            row.append(&icon_container);
        } else {
            let vpn_icon = gtk::Image::builder()
                .icon_name("network-vpn-symbolic")
                .pixel_size(20)
                .css_classes(["orbit-signal-icon"])
                .build();
            row.append(&vpn_icon);
        }
        let button_box = gtk::Box::builder()
            .orientation(Orientation::Horizontal)
            .halign(gtk::Align::End)
            .spacing(6)
            .build();
        
        let connect_button = gtk::Button::builder()
            .label("Connect")
            .css_classes(["orbit-button", "primary", "flat"])
            .build();
        button_box.append(&connect_button);
        
        let info_box = gtk::Box::builder()
            .orientation(Orientation::Vertical)
            .spacing(2)
            .hexpand(true)
            .valign(gtk::Align::Center)
            .build();
        
        let name = gtk::Label::builder()
            .label(&network.name)
            .css_classes(["orbit-ssid"])
            .halign(gtk::Align::Start)
            .build();
        info_box.append(&name);
        
        let status_text = if network.is_active {
            "Connected"
        } else if network.autoconnect {
            "Auto-connect enabled"
        } else {
            "Disconnected"
        };
        
        let status = gtk::Label::builder()
            .label(status_text)
            .css_classes(["orbit-status"])
            .halign(gtk::Align::Start)
            .build();
        info_box.append(&status);
        
        row.append(&info_box);
        row.append(&button_box);
        
        
        
        let path = network.path.clone();
        let is_user_action = Rc::new(RefCell::new(false));
        let is_user_action_clone = is_user_action.clone();
        
        glib::timeout_add_local(std::time::Duration::from_millis(100), move || {
            *is_user_action_clone.borrow_mut() = true;
            glib::ControlFlow::Break
        });
        
        
        let path = network.path.clone();
        row
    }
    
    pub fn widget(&self) -> &gtk::Box {
        &self.container
    }
    
    pub fn refresh_button(&self) -> &gtk::Button {
        &self.refresh_button
    }
    
}
