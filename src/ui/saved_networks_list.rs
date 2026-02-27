use gtk4::prelude::*;
use gtk4::{self as gtk, glib, Orientation};
use std::cell::RefCell;
use std::rc::Rc;
use crate::dbus::network_manager::SavedNetwork;

#[derive(Clone)]
pub struct SavedNetworksList {
    container: gtk::Box,
    list_box: gtk::Box,
    networks: Rc<RefCell<Vec<SavedNetwork>>>,
    on_autoconnect_toggle: Rc<RefCell<Option<Rc<dyn Fn(String, bool)>>>>,
    on_forget: Rc<RefCell<Option<Rc<dyn Fn(String)>>>>,
}

impl SavedNetworksList {
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
        
        let list = Self {
            container,
            list_box,
            networks: Rc::new(RefCell::new(Vec::new())),
            on_autoconnect_toggle: Rc::new(RefCell::new(None)),
            on_forget: Rc::new(RefCell::new(None)),
        };
        
        list.show_loading();
        list
    }
    
    fn show_loading(&self) {
        let placeholder = gtk::Label::builder()
            .label("Loading saved networks...")
            .css_classes(["orbit-placeholder"])
            .build();
        self.list_box.append(&placeholder);
    }
    
    fn show_placeholder(&self) {
        let placeholder = gtk::Label::builder()
            .label("No saved networks")
            .css_classes(["orbit-placeholder"])
            .build();
        self.list_box.append(&placeholder);
    }
    
    pub fn set_networks(&self, networks: Vec<SavedNetwork>) {
        *self.networks.borrow_mut() = networks.clone();
        
        while let Some(child) = self.list_box.first_child() {
            self.list_box.remove(&child);
        }
        
        if networks.is_empty() {
            self.show_placeholder();
            return;
        }
        
        let active_networks: Vec<&SavedNetwork> = networks.iter().filter(|n| n.is_active).collect();
        let saved_networks: Vec<&SavedNetwork> = networks.iter().filter(|n| !n.is_active).collect();
        
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
                .label("SAVED NETWORKS")
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
    
    fn create_network_row(&self, network: &SavedNetwork) -> gtk::Box {
        let css_classes = if network.is_active {
            vec!["orbit-saved-network-row", "active"]
        } else {
            vec!["orbit-saved-network-row"]
        };
        
        let row = gtk::Box::builder()
            .orientation(Orientation::Horizontal)
            .spacing(12)
            .css_classes(css_classes)
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
        
        if network.is_active {
            let icon_container = gtk::Box::builder()
                .css_classes(["orbit-icon-container"])
                .halign(gtk::Align::Center)
                .valign(gtk::Align::Center)
                .build();
            
            let wifi_icon = gtk::Image::builder()
                .icon_name("network-wireless-symbolic")
                .pixel_size(20)
                .css_classes(["orbit-icon-accent"])
                .build();
            icon_container.append(&wifi_icon);
            row.append(&icon_container);
        } else {
            let wifi_icon = gtk::Image::builder()
                .icon_name("network-wireless-symbolic")
                .pixel_size(20)
                .css_classes(["orbit-signal-icon"])
                .build();
            row.append(&wifi_icon);
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
        
        let status_text = if network.is_active {
            "Connected"
        } else if network.autoconnect {
            "Auto-connect enabled"
        } else {
            "Manual connect"
        };
        
        let status = gtk::Label::builder()
            .label(status_text)
            .css_classes(["orbit-status"])
            .halign(gtk::Align::Start)
            .build();
        info_box.append(&status);
        
        row.append(&info_box);
        
        let autoconnect_box = gtk::Box::builder()
            .orientation(Orientation::Horizontal)
            .spacing(4)
            .valign(gtk::Align::Center)
            .build();
        
        let auto_label = gtk::Label::builder()
            .label("Auto")
            .css_classes(["orbit-status"])
            .build();
        
        let autoconnect_switch = gtk::Switch::builder()
            .active(network.autoconnect)
            .css_classes(["orbit-toggle-switch"])
            .build();
        
        autoconnect_box.append(&auto_label);
        autoconnect_box.append(&autoconnect_switch);
        row.append(&autoconnect_box);
        
        // Forget Button
        let forget_btn = gtk::Button::builder()
            .label("Forget")
            .css_classes(["orbit-button", "destructive", "flat"])
            .valign(gtk::Align::Center)
            .build();
        row.append(&forget_btn);
        
        let path_toggle = network.path.clone();
        let on_toggle = self.on_autoconnect_toggle.clone();
        let is_user_action = Rc::new(RefCell::new(false));
        let is_user_action_clone = is_user_action.clone();
        
        glib::timeout_add_local(std::time::Duration::from_millis(100), move || {
            *is_user_action_clone.borrow_mut() = true;
            glib::ControlFlow::Break
        });
        
        autoconnect_switch.connect_state_notify(move |switch| {
            if *is_user_action.borrow() {
                if let Some(callback) = on_toggle.borrow().as_ref() {
                    callback(path_toggle.clone(), switch.is_active());
                }
            }
        });
        
        let path_forget = network.path.clone();
        let on_forget = self.on_forget.clone();
        forget_btn.connect_clicked(move |_| {
            if let Some(callback) = on_forget.borrow().as_ref() {
                callback(path_forget.clone());
            }
        });
        
        row
    }
    
    pub fn widget(&self) -> &gtk::Box {
        &self.container
    }
    
    pub fn set_on_autoconnect_toggle<F: Fn(String, bool) + 'static>(&self, callback: F) {
        *self.on_autoconnect_toggle.borrow_mut() = Some(Rc::new(callback));
    }

    pub fn set_on_forget<F: Fn(String) + 'static>(&self, callback: F) {
        *self.on_forget.borrow_mut() = Some(Rc::new(callback));
    }
}
