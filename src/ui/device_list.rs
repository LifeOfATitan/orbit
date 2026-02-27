use gtk4::prelude::*;
use gtk4::{self as gtk, Orientation};
use std::cell::RefCell;
use std::rc::Rc;
use std::collections::HashMap;
use crate::dbus::bluez::{BluetoothDevice, DeviceType};

#[derive(Clone)]
pub enum DeviceAction {
    Connect,
    Disconnect,
    Pair,
    Forget,
}

#[derive(Clone)]
pub struct DeviceList {
    container: gtk::Box,
    list_box: gtk::Box,
    scan_button: gtk::Button,
    devices: Rc<RefCell<Vec<BluetoothDevice>>>,
    row_actions: Rc<RefCell<HashMap<String, gtk::Box>>>,
    on_action: Rc<RefCell<Option<Rc<dyn Fn(String, DeviceAction)>>>>,
    action_path: Rc<RefCell<Option<String>>>,
    action_type: Rc<RefCell<Option<DeviceAction>>>,
}

impl DeviceList {
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
            .build();
        
        let scan_button = gtk::Button::builder()
            .label(" Scan for Devices")
            .css_classes(["orbit-button", "primary", "flat"])
            .hexpand(true)
            .build();
        
        footer.append(&scan_button);
        container.append(&footer);
        
        let list = Self {
            container,
            list_box,
            scan_button,
            devices: Rc::new(RefCell::new(Vec::new())),
            row_actions: Rc::new(RefCell::new(HashMap::new())),
            on_action: Rc::new(RefCell::new(None)),
            action_path: Rc::new(RefCell::new(None)),
            action_type: Rc::new(RefCell::new(None)),
        };
        
        list.show_loading();
        list
    }
    
    fn show_loading(&self) {
        let placeholder = gtk::Label::builder()
            .label("Loading devices...")
            .css_classes(["orbit-placeholder"])
            .build();
        self.list_box.append(&placeholder);
    }
    
    fn show_placeholder(&self) {
        let placeholder = gtk::Label::builder()
            .label("Click 'Scan' to find devices")
            .css_classes(["orbit-placeholder"])
            .build();
        self.list_box.append(&placeholder);
    }
    
    pub fn show_scanning(&self) {
        while let Some(child) = self.list_box.first_child() {
            self.list_box.remove(&child);
        }
        
        let scanning = gtk::Label::builder()
            .label("Scanning for devices...")
            .css_classes(["orbit-placeholder"])
            .build();
        self.list_box.append(&scanning);
    }

    pub fn set_action_state(&self, path: Option<String>, action: Option<DeviceAction>) {
        let old_path = self.action_path.borrow().clone();
        *self.action_path.borrow_mut() = path.clone();
        *self.action_type.borrow_mut() = action;
        
        if let Some(ref p) = path {
            self.update_single_row_actions(p);
        }
        if let Some(ref p) = old_path {
            self.update_single_row_actions(p);
        }
    }
    
    fn update_single_row_actions(&self, path: &str) {
        let devices = self.devices.borrow();
        if let Some(device) = devices.iter().find(|d| d.path == path) {
            let actions_map = self.row_actions.borrow();
            if let Some(actions_box) = actions_map.get(path) {
                while let Some(child) = actions_box.first_child() {
                    actions_box.remove(&child);
                }
                self.build_actions_box_content(actions_box, device);
            }
        }
    }
    
    pub fn set_devices(&self, devices: Vec<BluetoothDevice>) {
        *self.devices.borrow_mut() = devices.clone();
        *self.action_path.borrow_mut() = None;
        *self.action_type.borrow_mut() = None;
        self.render_devices(&devices);
    }
    
    fn render_devices(&self, devices: &[BluetoothDevice]) {
        self.row_actions.borrow_mut().clear();

        while let Some(child) = self.list_box.first_child() {
            self.list_box.remove(&child);
        }
        
        if devices.is_empty() {
            self.show_placeholder();
            return;
        }
        
        let connected_devices: Vec<&BluetoothDevice> = devices.iter().filter(|d| d.is_connected).collect();
        let paired_devices: Vec<&BluetoothDevice> = devices.iter().filter(|d| d.is_paired && !d.is_connected).collect();
        let available_devices: Vec<&BluetoothDevice> = devices.iter().filter(|d| !d.is_paired).collect();
        
        if !connected_devices.is_empty() {
            let section_header = gtk::Label::builder()
                .label("CONNECTED")
                .css_classes(["orbit-section-header"])
                .halign(gtk::Align::Start)
                .build();
            self.list_box.append(&section_header);
            
            for device in connected_devices {
                let row = self.create_device_row(device);
                self.list_box.append(&row);
            }
        }
        
        if !paired_devices.is_empty() {
            let section_header = gtk::Label::builder()
                .label("PAIRED")
                .css_classes(["orbit-section-header"])
                .halign(gtk::Align::Start)
                .build();
            self.list_box.append(&section_header);
            
            for device in paired_devices {
                let row = self.create_device_row(device);
                self.list_box.append(&row);
            }
        }
        
        if !available_devices.is_empty() {
            let section_header = gtk::Label::builder()
                .label("AVAILABLE")
                .css_classes(["orbit-section-header"])
                .halign(gtk::Align::Start)
                .build();
            self.list_box.append(&section_header);
            
            for device in available_devices {
                let row = self.create_device_row(device);
                self.list_box.append(&row);
            }
        }
    }
    
    fn create_device_row(&self, device: &BluetoothDevice) -> gtk::Box {
        let row = gtk::Box::builder()
            .orientation(Orientation::Horizontal)
            .spacing(12)
            .css_classes(["orbit-device-row"])
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

        let icon_name = match device.device_type {
            Some(DeviceType::Audio) => "audio-headphones-symbolic",
            Some(DeviceType::Keyboard) => "input-keyboard-symbolic",
            Some(DeviceType::Mouse) => "input-mouse-symbolic",
            Some(DeviceType::Phone) => "phone-symbolic",
            _ => "bluetooth-symbolic",
        };
        
        let icon = gtk::Image::builder()
            .icon_name(icon_name)
            .pixel_size(20)
            .css_classes(["orbit-device-icon"])
            .build();
        row.append(&icon);
        
        let info_box = gtk::Box::builder()
            .orientation(Orientation::Vertical)
            .spacing(2)
            .hexpand(true)
            .valign(gtk::Align::Center)
            .build();
        
        let name = gtk::Label::builder()
            .label(&device.name)
            .css_classes(["orbit-device-name"])
            .halign(gtk::Align::Start)
            .build();
        info_box.append(&name);
        
        let status_text = if device.is_connected {
            if let Some(ref battery) = device.battery_percentage {
                format!("Connected · {}%", battery)
            } else {
                "Connected".to_string()
            }
        } else if device.is_paired {
            "Paired".to_string()
        } else {
            "Available".to_string()
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
        
        self.build_actions_box_content(&actions_box, device);
        
        self.row_actions.borrow_mut().insert(device.path.clone(), actions_box.clone());
        
        row.append(&actions_box);
        row
    }

    fn build_actions_box_content(&self, actions_box: &gtk::Box, device: &BluetoothDevice) {
        let is_busy = self.action_path.borrow().as_deref() == Some(&device.path);
        
        if is_busy {
            let working_box = gtk::Box::builder()
                .orientation(Orientation::Horizontal)
                .spacing(8)
                .css_classes(["orbit-working-indicator"])
                .build();
            
            let spinner = gtk::Spinner::builder()
                .spinning(true)
                .build();
            spinner.start();
            
            let action_text = match self.action_type.borrow().as_ref() {
                Some(DeviceAction::Connect) => "Connecting...",
                Some(DeviceAction::Disconnect) => "Disconnecting...",
                Some(DeviceAction::Pair) => "Pairing...",
                Some(DeviceAction::Forget) => "Removing...",
                None => "Working...",
            };
            
            let label = gtk::Label::builder()
                .label(action_text)
                .css_classes(["orbit-status"])
                .build();
            
            working_box.append(&spinner);
            working_box.append(&label);
            actions_box.append(&working_box);
        } else {
            let (action_label, action) = if device.is_connected {
                ("Disconnect", DeviceAction::Disconnect)
            } else if device.is_paired {
                ("Connect", DeviceAction::Connect)
            } else {
                ("Pair", DeviceAction::Pair)
            };
            
            let action_btn = gtk::Button::builder()
                .label(action_label)
                .css_classes(if device.is_connected || device.is_paired {
                    vec!["orbit-button", "primary", "flat"]
                } else {
                    vec!["orbit-button", "flat"]
                })
                .build();
            
            let path = device.path.clone();
            let on_action = self.on_action.clone();
            action_btn.connect_clicked(move |_| {
                if let Some(callback) = on_action.borrow().as_ref() {
                    callback(path.clone(), action.clone());
                }
            });
            
            actions_box.append(&action_btn);
            
            if device.is_paired {
                let forget_btn = gtk::Button::builder()
                    .label("Forget")
                    .css_classes(["orbit-button", "destructive", "flat"])
                    .build();
                
                let path = device.path.clone();
                let on_action = self.on_action.clone();
                forget_btn.connect_clicked(move |_| {
                    if let Some(callback) = on_action.borrow().as_ref() {
                        callback(path.clone(), DeviceAction::Forget);
                    }
                });
                
                actions_box.append(&forget_btn);
            }
        }
    }
    
    pub fn widget(&self) -> &gtk::Box {
        &self.container
    }
    
    pub fn scan_button(&self) -> &gtk::Button {
        &self.scan_button
    }
    
    pub fn set_on_action<F: Fn(String, DeviceAction) + 'static>(&self, callback: F) {
        *self.on_action.borrow_mut() = Some(Rc::new(callback));
    }
}
