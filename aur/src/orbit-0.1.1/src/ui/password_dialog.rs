use gtk4::prelude::*;
use gtk4::{self as gtk, Orientation};

pub struct PasswordDialog {
    container: gtk::Box,
    ssid_label: gtk::Label,
    password_entry: gtk::PasswordEntry,
}

impl PasswordDialog {
    pub fn new() -> Self {
        let container = gtk::Box::builder()
            .orientation(Orientation::Vertical)
            .spacing(12)
            .margin_top(16)
            .margin_bottom(16)
            .margin_start(16)
            .margin_end(16)
            .build();
        
        let ssid_label = gtk::Label::builder()
            .halign(gtk::Align::Start)
            .build();
        container.append(&ssid_label);
        
        let prompt = gtk::Label::builder()
            .label("Enter password:")
            .halign(gtk::Align::Start)
            .build();
        container.append(&prompt);
        
        let password_entry = gtk::PasswordEntry::builder()
            .placeholder_text("Password")
            .show_peek_icon(true)
            .hexpand(true)
            .build();
        container.append(&password_entry);
        
        let button_box = gtk::Box::builder()
            .orientation(Orientation::Horizontal)
            .spacing(8)
            .halign(gtk::Align::End)
            .build();
        
        let cancel_button = gtk::Button::builder()
            .label("Cancel")
            .css_classes(["orbit-button", "secondary"])
            .build();
        
        let connect_button = gtk::Button::builder()
            .label("Connect")
            .css_classes(["orbit-button"])
            .build();
        
        button_box.append(&cancel_button);
        button_box.append(&connect_button);
        container.append(&button_box);
        
        Self {
            container,
            ssid_label,
            password_entry,
        }
    }
    
    pub fn set_ssid(&self, ssid: &str) {
        self.ssid_label.set_label(&format!("Connect to {}", ssid));
    }
    
    pub fn clear_password(&self) {
        self.password_entry.set_text("");
    }
    
    pub fn widget(&self) -> &gtk::Box {
        &self.container
    }
}
