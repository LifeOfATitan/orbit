use gtk4::prelude::*;
use gtk4::{self as gtk, Orientation};
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Clone)]
pub struct Header {
    container: gtk::Box,
    power_switch: gtk::Switch,
    is_programmatic_update: Rc<RefCell<bool>>,
}

impl Header {
    pub fn new() -> Self {
        let container = gtk::Box::builder()
            .orientation(Orientation::Vertical)
            .css_classes(["orbit-header"])
            .spacing(16)
            .build();
        
        let title_row = gtk::Box::builder()
            .orientation(Orientation::Horizontal)
            .spacing(10)
            .build();
        
        let orbit_icon = gtk::Image::builder()
            .icon_name("network-wireless-symbolic")
            .pixel_size(24)
            .css_classes(["orbit-icon-accent"])
            .build();
        
        let title = gtk::Label::builder()
            .label("Orbit")
            .css_classes(["orbit-title"])
            .hexpand(true)
            .halign(gtk::Align::Start)
            .build();
        
        let power_switch = gtk::Switch::builder()
            .css_classes(["orbit-toggle-switch"])
            .active(false)
            .sensitive(false)
            .build();
        
        let power_label = gtk::Label::builder()
            .label("WiFi")
            .css_classes(["orbit-status"])
            .build();
        
        let power_box = gtk::Box::builder()
            .orientation(Orientation::Horizontal)
            .spacing(8)
            .valign(gtk::Align::Center)
            .build();
        power_box.append(&power_label);
        power_box.append(&power_switch);
        
        title_row.append(&orbit_icon);
        title_row.append(&title);
        title_row.append(&power_box);
        
        let tab_bar = gtk::Box::builder()
            .orientation(Orientation::Horizontal)
            .css_classes(["orbit-tab-bar"])
            .homogeneous(true)
            .build();
        
        let wifi_tab = gtk::Button::builder()
            .label("WiFi")
            .css_classes(["orbit-tab", "flat", "active"])
            .hexpand(true)
            .build();
        
        let saved_tab = gtk::Button::builder()
            .label("Saved")
            .css_classes(["orbit-tab", "flat"])
            .hexpand(true)
            .build();
        
        let bluetooth_tab = gtk::Button::builder()
            .label("Bluetooth")
            .css_classes(["orbit-tab", "flat"])
            .hexpand(true)
            .build();
        
        tab_bar.append(&wifi_tab);
        tab_bar.append(&saved_tab);
        tab_bar.append(&bluetooth_tab);
        
        container.append(&title_row);
        container.append(&tab_bar);
        
        // Tab switching logic (local to Header)
        let wifi_tab_ref = wifi_tab.clone();
        let saved_tab_ref = saved_tab.clone();
        let bluetooth_tab_ref = bluetooth_tab.clone();
        let p_box = power_box.clone();
        let p_label = power_label.clone();

        wifi_tab.connect_clicked(move |_| {
            wifi_tab_ref.add_css_class("active");
            saved_tab_ref.remove_css_class("active");
            bluetooth_tab_ref.remove_css_class("active");
            p_box.set_visible(true);
            p_label.set_label("WiFi");
        });

        let wifi_tab_ref2 = wifi_tab.clone();
        let saved_tab_ref2 = saved_tab.clone();
        let bluetooth_tab_ref2 = bluetooth_tab.clone();
        let p_box2 = power_box.clone();

        saved_tab.connect_clicked(move |_| {
            wifi_tab_ref2.remove_css_class("active");
            saved_tab_ref2.add_css_class("active");
            bluetooth_tab_ref2.remove_css_class("active");
            p_box2.set_visible(false);
        });

        let wifi_tab_ref3 = wifi_tab.clone();
        let saved_tab_ref3 = saved_tab.clone();
        let bluetooth_tab_ref3 = bluetooth_tab.clone();
        let p_box3 = power_box.clone();
        let p_label3 = power_label.clone();

        bluetooth_tab.connect_clicked(move |_| {
            wifi_tab_ref3.remove_css_class("active");
            saved_tab_ref3.remove_css_class("active");
            bluetooth_tab_ref3.add_css_class("active");
            p_box3.set_visible(true);
            p_label3.set_label("Bluetooth");
        });

        Self {
            container,
            power_switch,
            is_programmatic_update: Rc::new(RefCell::new(false)),
        }
    }
    
    pub fn widget(&self) -> &gtk::Box {
        &self.container
    }
    
    pub fn set_power_state(&self, enabled: bool) {
        *self.is_programmatic_update.borrow_mut() = true;
        self.power_switch.set_sensitive(true);
        self.power_switch.set_active(enabled);
        *self.is_programmatic_update.borrow_mut() = false;
    }
    
    pub fn power_switch(&self) -> &gtk::Switch {
        &self.power_switch
    }
}
