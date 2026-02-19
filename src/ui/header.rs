use gtk4::prelude::*;
use gtk4::{self as gtk, Orientation};
use std::cell::RefCell;
use std::rc::Rc;
use crate::theme::Theme;

#[derive(Clone)]
pub struct Header {
    container: gtk::Box,
    wifi_tab: gtk::Button,
    saved_tab: gtk::Button,
    bluetooth_tab: gtk::Button,
    power_switch: gtk::Switch,
    power_box: gtk::Box,
    theme: Rc<RefCell<Theme>>,
    is_programmatic_update: Rc<RefCell<bool>>,
}

impl Header {
    pub fn new(theme: Rc<RefCell<Theme>>) -> Self {
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
            .active(true)
            .build();
        
        let power_box = gtk::Box::builder()
            .orientation(Orientation::Horizontal)
            .build();
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
        
        let header = Self {
            container,
            wifi_tab,
            saved_tab,
            bluetooth_tab,
            power_switch,
            power_box,
            theme,
            is_programmatic_update: Rc::new(RefCell::new(false)),
        };

        header
    }
    
    pub fn widget(&self) -> &gtk::Box {
        &self.container
    }
    
    pub fn set_power_state(&self, enabled: bool) {
        *self.is_programmatic_update.borrow_mut() = true;
        self.power_switch.set_active(enabled);
        *self.is_programmatic_update.borrow_mut() = false;
    }
    
    pub fn is_programmatic_update(&self) -> Rc<RefCell<bool>> {
        self.is_programmatic_update.clone()
    }
    
    pub fn wifi_tab(&self) -> &gtk::Button {
        &self.wifi_tab
    }
    
    pub fn saved_tab(&self) -> &gtk::Button {
        &self.saved_tab
    }

    pub fn bluetooth_tab(&self) -> &gtk::Button {
        &self.bluetooth_tab
    }

    pub fn power_switch(&self) -> &gtk::Switch {
        &self.power_switch
    }

    pub fn set_tab(&self, tab: &str) {
        self.wifi_tab.remove_css_class("active");
        self.saved_tab.remove_css_class("active");
        self.bluetooth_tab.remove_css_class("active");

        match tab {
            "wifi" => {
                self.wifi_tab.add_css_class("active");
                self.power_box.set_visible(true);
            }
            "saved" => {
                self.saved_tab.add_css_class("active");
                self.power_box.set_visible(false);
            }
            "bluetooth" => {
                self.bluetooth_tab.add_css_class("active");
                self.power_box.set_visible(true);
            }
            _ => {}
        }
    }
}
