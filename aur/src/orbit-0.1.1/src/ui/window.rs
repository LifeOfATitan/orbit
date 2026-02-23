use gtk4::{ApplicationWindow, Application, prelude::*, Overlay};
use gtk4::{self as gtk, Orientation};
use gtk4_layer_shell::{LayerShell, Layer, KeyboardMode, Edge};
use std::cell::RefCell;
use std::rc::Rc;
use std::path::Path;
use std::fs;
use gtk4::style_context_add_provider_for_display;

use crate::config::Config;
use crate::theme::Theme;
use super::header::Header;
use super::network_list::NetworkList;
use super::device_list::DeviceList;
use super::saved_networks_list::SavedNetworksList;

pub struct OrbitWindow {
    window: ApplicationWindow,
    header: Header,
    network_list: NetworkList,
    saved_networks_list: SavedNetworksList,
    device_list: DeviceList,
    stack: gtk::Stack,
    details_revealer: gtk::Revealer,
    details_box: gtk::Box,
    details_content: gtk::Box,
    password_revealer: gtk::Revealer,
    password_box: gtk::Box,
    password_entry: gtk::PasswordEntry,
    password_label: gtk::Label,
    password_callback: Rc<RefCell<Option<Box<dyn Fn(Option<String>)>>>>,
    error_revealer: gtk::Revealer,
    error_box: gtk::Box,
    error_label: gtk::Label,
    theme: Rc<RefCell<Theme>>,
    css_provider: gtk4::CssProvider,
    user_css_provider: gtk4::CssProvider,
}

impl Clone for OrbitWindow {
    fn clone(&self) -> Self {
        Self {
            window: self.window.clone(),
            header: self.header.clone(),
            network_list: self.network_list.clone(),
            saved_networks_list: self.saved_networks_list.clone(),
            device_list: self.device_list.clone(),
            stack: self.stack.clone(),
            details_revealer: self.details_revealer.clone(),
            details_box: self.details_box.clone(),
            details_content: self.details_content.clone(),
            password_revealer: self.password_revealer.clone(),
            password_box: self.password_box.clone(),
            password_entry: self.password_entry.clone(),
            password_label: self.password_label.clone(),
            password_callback: self.password_callback.clone(),
            error_revealer: self.error_revealer.clone(),
            error_box: self.error_box.clone(),
            error_label: self.error_label.clone(),
            theme: self.theme.clone(),
            css_provider: self.css_provider.clone(),
            user_css_provider: self.user_css_provider.clone(),
        }
    }
}

impl OrbitWindow {
    pub fn new(app: &Application, config: Config, theme: Rc<RefCell<Theme>>) -> Self {
        let window = ApplicationWindow::builder()
            .application(app)
            .default_width(420)
            .default_height(500)
            .resizable(false)
            .decorated(false)
            .build();
        
        window.init_layer_shell();
        window.set_namespace("orbit");
        window.set_exclusive_zone(0);
        window.set_layer(Layer::Overlay);
        window.set_keyboard_mode(KeyboardMode::OnDemand);
        window.set_default_size(420, 500);
        
        window.add_css_class("background");
        
        let css_provider = gtk4::CssProvider::new();
        let user_css_provider = gtk4::CssProvider::new();
        
        let display = gtk4::gdk::Display::default().expect("Failed to get default display");
        gtk4::style_context_add_provider_for_display(
            &display,
            &css_provider,
            gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );

        gtk4::style_context_add_provider_for_display(
            &display,
            &user_css_provider,
            gtk4::STYLE_PROVIDER_PRIORITY_USER,
        );

        let (col, row) = config.position_tuple();
        
        match (col, row) {
            (0, 0) => {
                window.set_anchor(Edge::Top, true);
                window.set_anchor(Edge::Left, true);
                window.set_margin(Edge::Top, 10);
                window.set_margin(Edge::Left, 10);
            }
            (1, 0) => {
                window.set_anchor(Edge::Top, true);
                window.set_margin(Edge::Top, 10);
            }
            (2, 0) => {
                window.set_anchor(Edge::Top, true);
                window.set_anchor(Edge::Right, true);
                window.set_margin(Edge::Top, 10);
                window.set_margin(Edge::Right, 10);
            }
            (0, 1) => {
                window.set_anchor(Edge::Left, true);
                window.set_margin(Edge::Left, 10);
            }
            (1, 1) => {}
            (2, 1) => {
                window.set_anchor(Edge::Right, true);
                window.set_margin(Edge::Right, 10);
            }
            (0, 2) => {
                window.set_anchor(Edge::Bottom, true);
                window.set_anchor(Edge::Left, true);
                window.set_margin(Edge::Bottom, 10);
                window.set_margin(Edge::Left, 10);
            }
            (1, 2) => {
                window.set_anchor(Edge::Bottom, true);
                window.set_margin(Edge::Bottom, 10);
            }
            (2, 2) => {
                window.set_anchor(Edge::Bottom, true);
                window.set_anchor(Edge::Right, true);
                window.set_margin(Edge::Bottom, 10);
                window.set_margin(Edge::Right, 10);
            }
            _ => {}
        }
        
        let main_box = gtk::Box::builder()
            .orientation(Orientation::Vertical)
            .css_classes(["orbit-panel"])
            .vexpand(true)
            .hexpand(true)
            .overflow(gtk::Overflow::Hidden)
            .build();
        
        let header = Header::new(theme.clone());
        main_box.append(header.widget());
        
        let stack = gtk::Stack::builder()
            .vexpand(true)
            .hexpand(true)
            .build();
        
        let network_list = NetworkList::new(theme.clone());
        let saved_networks_list = SavedNetworksList::new(theme.clone());
        let device_list = DeviceList::new(theme.clone());
        
        stack.add_named(network_list.widget(), Some("wifi"));
        stack.add_named(saved_networks_list.widget(), Some("saved"));
        stack.add_named(device_list.widget(), Some("bluetooth"));
        stack.set_visible_child_name("wifi");
        stack.set_size_request(400, 350);
        
        main_box.append(&stack);
        
        let overlay = Overlay::new();
        overlay.set_child(Some(&main_box));
        
        let details_box = gtk::Box::builder()
            .orientation(Orientation::Vertical)
            .css_classes(["orbit-details-overlay"])
            .spacing(8)
            .margin_start(16)
            .margin_end(16)
            .margin_top(16)
            .margin_bottom(16)
            .build();
        
        let details_content = gtk::Box::builder()
            .orientation(Orientation::Vertical)
            .spacing(4)
            .build();
        
        let close_btn = gtk::Button::builder()
            .label("Close")
            .css_classes(["orbit-button", "primary"])
            .halign(gtk::Align::Center)
            .margin_top(4)
            .build();
        
        details_box.append(&details_content);
        details_box.append(&close_btn);
        
        let details_revealer = gtk::Revealer::builder()
            .child(&details_box)
            .reveal_child(false)
            .transition_type(gtk::RevealerTransitionType::SlideUp)
            .transition_duration(250)
            .valign(gtk::Align::End)
            .can_target(true)
            .build();
        
        let details_revealer_clone = details_revealer.clone();
        close_btn.connect_clicked(move |_| {
            details_revealer_clone.set_reveal_child(false);
        });
        
        overlay.add_overlay(&details_revealer);
        
        let password_box = gtk::Box::builder()
            .orientation(Orientation::Vertical)
            .spacing(12)
            .css_classes(["orbit-password-overlay"])
            .margin_start(16)
            .margin_end(16)
            .margin_top(16)
            .margin_bottom(16)
            .build();
        
        let password_label = gtk::Label::builder()
            .label("Enter WiFi password:")
            .css_classes(["orbit-detail-label"])
            .halign(gtk::Align::Start)
            .build();
        
        let password_entry = gtk::PasswordEntry::builder()
            .placeholder_text("Password")
            .show_peek_icon(true)
            .hexpand(true)
            .build();
        
        let password_btn_row = gtk::Box::builder()
            .orientation(Orientation::Horizontal)
            .spacing(8)
            .halign(gtk::Align::End)
            .build();
        
        let password_cancel_btn = gtk::Button::builder()
            .label("Cancel")
            .css_classes(["orbit-button"])
            .build();
        
        let password_connect_btn = gtk::Button::builder()
            .label("Connect")
            .css_classes(["orbit-button", "primary"])
            .build();
        
        password_btn_row.append(&password_cancel_btn);
        password_btn_row.append(&password_connect_btn);
        
        password_box.append(&password_label);
        password_box.append(&password_entry);
        password_box.append(&password_btn_row);
        
        let password_revealer = gtk::Revealer::builder()
            .child(&password_box)
            .reveal_child(false)
            .transition_type(gtk::RevealerTransitionType::SlideUp)
            .transition_duration(250)
            .valign(gtk::Align::End)
            .can_target(true)
            .build();
        
        overlay.add_overlay(&password_revealer);
        
        let error_box = gtk::Box::builder()
            .orientation(Orientation::Horizontal)
            .spacing(12)
            .css_classes(["orbit-error-overlay"])
            .margin_start(16)
            .margin_end(16)
            .margin_top(16)
            .margin_bottom(16)
            .build();
        
        let error_icon = gtk::Image::builder()
            .icon_name("dialog-warning-symbolic")
            .pixel_size(20)
            .build();
        
        let error_label = gtk::Label::builder()
            .label("")
            .css_classes(["orbit-error-label"])
            .wrap(true)
            .hexpand(true)
            .halign(gtk::Align::Start)
            .build();
        
        let error_close_btn = gtk::Button::builder()
            .label("Dismiss")
            .css_classes(["orbit-button", "destructive"])
            .build();
        
        error_box.append(&error_icon);
        error_box.append(&error_label);
        error_box.append(&error_close_btn);
        
        let error_revealer = gtk::Revealer::builder()
            .child(&error_box)
            .reveal_child(false)
            .transition_type(gtk::RevealerTransitionType::SlideUp)
            .transition_duration(250)
            .valign(gtk::Align::End)
            .can_target(true)
            .build();
        
        let error_revealer_clone = error_revealer.clone();
        error_close_btn.connect_clicked(move |_| {
            error_revealer_clone.set_reveal_child(false);
        });
        
        overlay.add_overlay(&error_revealer);
        
        window.set_child(Some(&overlay));
        
        let password_callback: Rc<RefCell<Option<Box<dyn Fn(Option<String>)>>>> = Rc::new(RefCell::new(None));
        
        let password_revealer_clone = password_revealer.clone();
        let password_entry_clone = password_entry.clone();
        let password_callback_clone = password_callback.clone();
        password_connect_btn.connect_clicked(move |_| {
            let pw = password_entry_clone.text().to_string();
            let password = if pw.is_empty() { None } else { Some(pw) };
            password_entry_clone.set_text("");
            password_revealer_clone.set_reveal_child(false);
            if let Some(cb) = password_callback_clone.borrow_mut().take() {
                cb(password);
            }
        });
        
        let password_revealer_clone2 = password_revealer.clone();
        let password_entry_clone2 = password_entry.clone();
        let password_callback_clone2 = password_callback.clone();
        password_cancel_btn.connect_clicked(move |_| {
            password_entry_clone2.set_text("");
            password_revealer_clone2.set_reveal_child(false);
            if let Some(cb) = password_callback_clone2.borrow_mut().take() {
                cb(None);
            }
        });
        
        let win = Self {
            window: window.clone(),
            header,
            network_list,
            saved_networks_list,
            device_list,
            stack,
            details_revealer,
            details_box,
            details_content,
            password_revealer,
            password_box,
            password_entry,
            password_label,
            password_callback,
            error_revealer,
            error_box,
            error_label,
            theme,
            css_provider,
            user_css_provider,
        };

        // Add Escape key shortcut to hide the window
        let key_controller = gtk::EventControllerKey::new();
        let win_clone = win.clone();
        key_controller.connect_key_pressed(move |_, key, _, _| {
            if key == gtk4::gdk::Key::Escape {
                // Hide overlays first if they are visible, otherwise hide window
                if win_clone.details_revealer.reveals_child() {
                    win_clone.details_revealer.set_reveal_child(false);
                } else if win_clone.password_revealer.reveals_child() {
                    win_clone.hide_password_dialog();
                } else if win_clone.error_revealer.reveals_child() {
                    win_clone.error_revealer.set_reveal_child(false);
                } else {
                    win_clone.hide();
                }
                gtk4::glib::Propagation::Stop
            } else {
                gtk4::glib::Propagation::Proceed
            }
        });
        window.add_controller(key_controller);
        
        win.apply_theme();
        
        win
    }
    
    pub fn apply_theme(&self) {
        let css = self.theme.borrow().generate_css();
        self.css_provider.load_from_data(&css);

        if let Ok(home) = std::env::var("HOME") {
            let user_css_path = format!("{}/.config/orbit/style.css", home);
            let path = Path::new(&user_css_path);

            if path.exists() {
                self.user_css_provider.load_from_path(path);
            } else {
                self.user_css_provider.load_from_data("");
            }
        }
    }
    
    pub fn show(&self) {
        self.window.set_keyboard_mode(KeyboardMode::OnDemand);
        self.window.present();
    }
    
    pub fn hide(&self) {
        self.window.set_visible(false);
        self.window.set_keyboard_mode(KeyboardMode::None);
    }
    
    pub fn network_list(&self) -> &NetworkList {
        &self.network_list
    }
    
    pub fn device_list(&self) -> &DeviceList {
        &self.device_list
    }

    pub fn saved_networks_list(&self) -> &SavedNetworksList {
        &self.saved_networks_list
    }
    
    pub fn header(&self) -> &Header {
        &self.header
    }
    
    pub fn stack(&self) -> &gtk::Stack {
        &self.stack
    }
    
    pub fn window(&self) -> &gtk::ApplicationWindow {
        &self.window
    }

    pub fn set_position(&self, position: &str) {
        // Reset all horizontal/vertical anchors first to avoid conflicts
        self.window.set_anchor(Edge::Top, false);
        self.window.set_anchor(Edge::Bottom, false);
        self.window.set_anchor(Edge::Left, false);
        self.window.set_anchor(Edge::Right, false);

        match position {
            "top-left" => {
                self.window.set_anchor(Edge::Top, true);
                self.window.set_anchor(Edge::Left, true);
                self.window.set_margin(Edge::Top, 10);
                self.window.set_margin(Edge::Left, 10);
            }
            "top" => {
                self.window.set_anchor(Edge::Top, true);
                self.window.set_margin(Edge::Top, 10);
            }
            "top-right" => {
                self.window.set_anchor(Edge::Top, true);
                self.window.set_anchor(Edge::Right, true);
                self.window.set_margin(Edge::Top, 10);
                self.window.set_margin(Edge::Right, 10);
            }
            "bottom-left" => {
                self.window.set_anchor(Edge::Bottom, true);
                self.window.set_anchor(Edge::Left, true);
                self.window.set_margin(Edge::Bottom, 10);
                self.window.set_margin(Edge::Left, 10);
            }
            "bottom" => {
                self.window.set_anchor(Edge::Bottom, true);
                self.window.set_margin(Edge::Bottom, 10);
            }
            "bottom-right" => {
                self.window.set_anchor(Edge::Bottom, true);
                self.window.set_anchor(Edge::Right, true);
                self.window.set_margin(Edge::Bottom, 10);
                self.window.set_margin(Edge::Right, 10);
            }
            _ => {
                // Default to top-right if unknown
                self.window.set_anchor(Edge::Top, true);
                self.window.set_anchor(Edge::Right, true);
            }
        }
    }
    
    pub fn show_password_dialog<F: Fn(Option<String>) + 'static>(&self, ssid: &str, callback: F) {
        self.details_revealer.set_reveal_child(false);
        self.password_label.set_label(&format!("Enter password for {}:", ssid));
        self.password_entry.set_text("");
        *self.password_callback.borrow_mut() = Some(Box::new(callback));
        self.password_revealer.set_reveal_child(true);
        self.password_entry.grab_focus();
    }
    
    pub fn hide_password_dialog(&self) {
        self.password_entry.set_text("");
        self.password_revealer.set_reveal_child(false);
        *self.password_callback.borrow_mut() = None;
    }
    
    pub fn show_error(&self, message: &str) {
        self.details_revealer.set_reveal_child(false);
        self.password_revealer.set_reveal_child(false);
        self.error_label.set_label(message);
        self.error_revealer.set_reveal_child(true);
    }
    
    pub fn hide_error(&self) {
        self.error_revealer.set_reveal_child(false);
    }
    
    pub fn show_network_details(&self, details: &crate::dbus::network_manager::NetworkDetails) {
        while let Some(child) = self.details_content.first_child() {
            self.details_content.remove(&child);
        }
        
        let dns_text = if details.dns_servers.is_empty() {
            "N/A".to_string()
        } else {
            details.dns_servers.join(", ")
        };
        
        let ip_text = if details.ip4_address.is_empty() { "N/A" } else { details.ip4_address.as_str() };
        let gateway_text = if details.gateway.is_empty() { "N/A" } else { details.gateway.as_str() };
        let mac_text = if details.mac_address.is_empty() { "N/A" } else { details.mac_address.as_str() };
        let speed_text = if details.connection_speed.is_empty() { "N/A" } else { details.connection_speed.as_str() };
        
        let rows: [(&str, &str, &str); 6] = [
            ("SSID", details.ssid.as_str(), "network-wireless-symbolic"),
            ("IP Address", ip_text, "network-server-symbolic"),
            ("Gateway", gateway_text, "network-server-symbolic"),
            ("DNS", dns_text.as_str(), "web-browser-symbolic"),
            ("MAC Address", mac_text, "dialog-password-symbolic"),
            ("Speed", speed_text, "network-transmit-receive-symbolic"),
        ];
        
        for (label, value, icon_name) in rows {
            let row = gtk::Box::builder()
                .orientation(Orientation::Horizontal)
                .css_classes(["orbit-details-row"])
                .spacing(8)
                .build();
            
            let icon = gtk::Image::builder()
                .icon_name(icon_name)
                .pixel_size(16)
                .css_classes(["orbit-detail-icon"])
                .build();
            
            let label_widget = gtk::Label::builder()
                .label(label)
                .css_classes(["orbit-detail-label"])
                .halign(gtk::Align::Start)
                .hexpand(true)
                .build();
            
            let value_widget = gtk::Label::builder()
                .label(value)
                .css_classes(["orbit-detail-value"])
                .halign(gtk::Align::End)
                .build();
            
            row.append(&icon);
            row.append(&label_widget);
            row.append(&value_widget);
            self.details_content.append(&row);
        }
        
        self.password_revealer.set_reveal_child(false);
        self.error_revealer.set_reveal_child(false);
        self.details_revealer.set_reveal_child(true);
    }
}
