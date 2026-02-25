use gtk4::{ApplicationWindow, Application, prelude::*, Overlay};
use gtk4::{self as gtk, Orientation};
use gtk4_layer_shell::{LayerShell, Layer, KeyboardMode, Edge};
use std::cell::RefCell;
use std::rc::Rc;

use crate::config::Config;
use crate::theme::Theme;
use super::header::Header;
use super::network_list::NetworkList;
use super::device_list::DeviceList;
use super::saved_networks_list::SavedNetworksList;

pub struct OrbitWindow {
    window: ApplicationWindow,
    config: Rc<RefCell<Config>>,
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
    password_error_label: gtk::Label,
    password_connect_btn: gtk::Button,
    password_callback: Rc<RefCell<Option<Rc<dyn Fn(Option<String>)>>>>,
    hidden_revealer: gtk::Revealer,
    hidden_ssid_entry: gtk::Entry,
    hidden_password_entry: gtk::PasswordEntry,
    hidden_callback: Rc<RefCell<Option<Rc<dyn Fn(Option<(String, String)>)>>>>,
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
            config: self.config.clone(),
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
            password_error_label: self.password_error_label.clone(),
            password_connect_btn: self.password_connect_btn.clone(),
            password_callback: self.password_callback.clone(),
            hidden_revealer: self.hidden_revealer.clone(),
            hidden_ssid_entry: self.hidden_ssid_entry.clone(),
            hidden_password_entry: self.hidden_password_entry.clone(),
            hidden_callback: self.hidden_callback.clone(),
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
        window.set_layer(Layer::Overlay);
        window.set_keyboard_mode(KeyboardMode::None);
        window.set_exclusive_zone(0);
        window.set_default_size(420, 500);
        
        window.add_css_class("background");
        
        let css_provider = gtk4::CssProvider::new();
        let user_css_provider = gtk4::CssProvider::new();
        
        let display = gtk4::gdk::Display::default().expect("Failed to get default display");
        gtk4::style_context_add_provider_for_display(
            &display,
            &css_provider,
            gtk4::STYLE_PROVIDER_PRIORITY_USER,
        );

        gtk4::style_context_add_provider_for_display(
            &display,
            &user_css_provider,
            gtk4::STYLE_PROVIDER_PRIORITY_USER,
        );

        let config = Rc::new(RefCell::new(config));

        let main_box = gtk::Box::builder()
            .orientation(Orientation::Vertical)
            .css_classes(["orbit-panel"])
            .vexpand(true)
            .hexpand(true)
            .overflow(gtk::Overflow::Hidden)
            .build();
        
        let header = Header::new();
        main_box.append(header.widget());
        
        let stack = gtk::Stack::builder()
            .vexpand(true)
            .hexpand(true)
            .build();
        
        let network_list = NetworkList::new();
        let saved_networks_list = SavedNetworksList::new();
        let device_list = DeviceList::new();
        
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
            .css_classes(["orbit-button", "primary", "flat"])
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
        
        let password_error_label = gtk::Label::builder()
            .label("")
            .css_classes(["orbit-password-error"])
            .halign(gtk::Align::Start)
            .wrap(true)
            .wrap_mode(gtk::pango::WrapMode::WordChar)
            .max_width_chars(40)
            .visible(false)
            .build();
        
        let password_btn_row = gtk::Box::builder()
            .orientation(Orientation::Horizontal)
            .spacing(8)
            .halign(gtk::Align::End)
            .build();
        
        let password_cancel_btn = gtk::Button::builder()
            .label("Cancel")
            .css_classes(["orbit-button", "flat"])
            .build();
        
        let password_connect_btn = gtk::Button::builder()
            .label("Connect")
            .css_classes(["orbit-button", "primary", "flat"])
            .build();
        
        password_btn_row.append(&password_cancel_btn);
        password_btn_row.append(&password_connect_btn);
        
        password_box.append(&password_label);
        password_box.append(&password_entry);
        password_box.append(&password_error_label);
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

        let hidden_box = gtk::Box::builder()
            .orientation(Orientation::Vertical)
            .spacing(12)
            .css_classes(["orbit-password-overlay"])
            .margin_start(16)
            .margin_end(16)
            .margin_top(16)
            .margin_bottom(16)
            .build();
        
        let hidden_label = gtk::Label::builder()
            .label("Connect to Hidden Network:")
            .css_classes(["orbit-detail-label"])
            .halign(gtk::Align::Start)
            .build();
        
        let hidden_ssid_entry = gtk::Entry::builder()
            .placeholder_text("Network SSID")
            .hexpand(true)
            .build();

        let hidden_password_entry = gtk::PasswordEntry::builder()
            .placeholder_text("Password (optional)")
            .show_peek_icon(true)
            .hexpand(true)
            .build();
        
        let hidden_btn_row = gtk::Box::builder()
            .orientation(Orientation::Horizontal)
            .spacing(8)
            .halign(gtk::Align::End)
            .build();
        
        let hidden_cancel_btn = gtk::Button::builder()
            .label("Cancel")
            .css_classes(["orbit-button", "flat"])
            .build();
        
        let hidden_connect_btn = gtk::Button::builder()
            .label("Connect")
            .css_classes(["orbit-button", "primary", "flat"])
            .build();
        
        hidden_btn_row.append(&hidden_cancel_btn);
        hidden_btn_row.append(&hidden_connect_btn);
        
        hidden_box.append(&hidden_label);
        hidden_box.append(&hidden_ssid_entry);
        hidden_box.append(&hidden_password_entry);
        hidden_box.append(&hidden_btn_row);
        
        let hidden_revealer = gtk::Revealer::builder()
            .child(&hidden_box)
            .reveal_child(false)
            .transition_type(gtk::RevealerTransitionType::SlideUp)
            .transition_duration(250)
            .valign(gtk::Align::End)
            .can_target(true)
            .build();
        
        overlay.add_overlay(&hidden_revealer);
        
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
            .css_classes(["orbit-button", "destructive", "flat"])
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
        
        let password_callback: Rc<RefCell<Option<Rc<dyn Fn(Option<String>)>>>> = Rc::new(RefCell::new(None));
        let hidden_callback: Rc<RefCell<Option<Rc<dyn Fn(Option<(String, String)>)>>>> = Rc::new(RefCell::new(None));

        let hidden_ssid_entry_clone = hidden_ssid_entry.clone();
        let hidden_password_entry_clone = hidden_password_entry.clone();
        let hidden_callback_clone = hidden_callback.clone();
        let hidden_revealer_clone = hidden_revealer.clone();
        hidden_connect_btn.connect_clicked(move |_| {
            let ssid = hidden_ssid_entry_clone.text().to_string();
            let pw = hidden_password_entry_clone.text().to_string();
            if ssid.is_empty() {
                return;
            }
            hidden_revealer_clone.set_reveal_child(false);
            if let Some(ref cb) = *hidden_callback_clone.borrow() {
                cb(Some((ssid, pw)));
            }
        });

        let hidden_ssid_entry_cancel = hidden_ssid_entry.clone();
        let hidden_password_entry_cancel = hidden_password_entry.clone();
        let hidden_callback_cancel = hidden_callback.clone();
        let hidden_revealer_cancel = hidden_revealer.clone();
        hidden_cancel_btn.connect_clicked(move |_| {
            hidden_ssid_entry_cancel.set_text("");
            hidden_password_entry_cancel.set_text("");
            hidden_revealer_cancel.set_reveal_child(false);
            if let Some(cb) = hidden_callback_cancel.borrow_mut().take() {
                cb(None);
            }
        });

        // Enter-to-submit in hidden network entries
        let hidden_ssid_activate = hidden_ssid_entry.clone();
        let hidden_password_activate = hidden_password_entry.clone();
        let hidden_callback_activate = hidden_callback.clone();
        let hidden_revealer_activate = hidden_revealer.clone();
        hidden_ssid_entry.connect_activate(move |_| {
            let ssid = hidden_ssid_activate.text().to_string();
            let pw = hidden_password_activate.text().to_string();
            if ssid.is_empty() { return; }
            hidden_revealer_activate.set_reveal_child(false);
            if let Some(ref cb) = *hidden_callback_activate.borrow() {
                cb(Some((ssid, pw)));
            }
        });

        let hidden_ssid_activate_pw = hidden_ssid_entry.clone();
        let hidden_password_activate_pw = hidden_password_entry.clone();
        let hidden_callback_activate_pw = hidden_callback.clone();
        let hidden_revealer_activate_pw = hidden_revealer.clone();
        hidden_password_entry.connect_activate(move |_| {
            let ssid = hidden_ssid_activate_pw.text().to_string();
            let pw = hidden_password_activate_pw.text().to_string();
            if ssid.is_empty() { return; }
            hidden_revealer_activate_pw.set_reveal_child(false);
            if let Some(ref cb) = *hidden_callback_activate_pw.borrow() {
                cb(Some((ssid, pw)));
            }
        });

        let password_entry_clone = password_entry.clone();
        let password_callback_clone = password_callback.clone();
        let password_connect_btn_clone = password_connect_btn.clone();
        let password_error_label_clone = password_error_label.clone();
        password_connect_btn.connect_clicked(move |_| {
            let pw = password_entry_clone.text().to_string();
            if pw.is_empty() {
                password_error_label_clone.set_label("Password cannot be empty");
                password_error_label_clone.set_visible(true);
                return;
            }
            // Set connecting state - don't close dialog
            password_connect_btn_clone.set_label("Connecting...");
            password_connect_btn_clone.set_sensitive(false);
            password_error_label_clone.set_visible(false);
            if let Some(ref cb) = *password_callback_clone.borrow() {
                cb(Some(pw));
            }
        });
        
        let password_revealer_clone2 = password_revealer.clone();
        let password_entry_clone2 = password_entry.clone();
        let password_callback_clone2 = password_callback.clone();
        let password_error_label_clone2 = password_error_label.clone();
        let password_connect_btn_clone2 = password_connect_btn.clone();
        password_cancel_btn.connect_clicked(move |_| {
            password_entry_clone2.set_text("");
            password_revealer_clone2.set_reveal_child(false);
            password_error_label_clone2.set_visible(false);
            password_connect_btn_clone2.set_label("Connect");
            password_connect_btn_clone2.set_sensitive(true);
            if let Some(cb) = password_callback_clone2.borrow_mut().take() {
                cb(None);
            }
        });
        
        // Enter-to-submit in password entry
        let password_entry_activate = password_entry.clone();
        let password_callback_activate = password_callback.clone();
        let password_connect_btn_activate = password_connect_btn.clone();
        let password_error_label_activate = password_error_label.clone();
        password_entry.connect_activate(move |_| {
            let pw = password_entry_activate.text().to_string();
            if pw.is_empty() {
                password_error_label_activate.set_label("Password cannot be empty");
                password_error_label_activate.set_visible(true);
                return;
            }
            password_connect_btn_activate.set_label("Connecting...");
            password_connect_btn_activate.set_sensitive(false);
            password_error_label_activate.set_visible(false);
            if let Some(ref cb) = *password_callback_activate.borrow() {
                cb(Some(pw));
            }
        });
        
        let win = Self {
            window: window.clone(),
            config,
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
            password_error_label,
            password_connect_btn: password_connect_btn.clone(),
            password_callback,
            hidden_revealer,
            hidden_ssid_entry,
            hidden_password_entry,
            hidden_callback,
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
                } else if win_clone.hidden_revealer.reveals_child() {
                    win_clone.hidden_revealer.set_reveal_child(false);
                } else if win_clone.error_revealer.reveals_child() {
                    win_clone.error_revealer.set_reveal_child(false);
                } else {
                    win_clone.hide();
                }
                gtk4::glib::Propagation::Stop
            } else if key == gtk4::gdk::Key::Down || key == gtk4::gdk::Key::Tab {
                win_clone.window.child_focus(gtk::DirectionType::TabForward);
                gtk4::glib::Propagation::Stop
            } else if key == gtk4::gdk::Key::Up || key == gtk4::gdk::Key::ISO_Left_Tab {
                win_clone.window.child_focus(gtk::DirectionType::TabBackward);
                gtk4::glib::Propagation::Stop
            } else {
                gtk4::glib::Propagation::Proceed
            }
        });
        window.add_controller(key_controller);
        
        
        win.apply_position();
        win.apply_theme();
        
        win
    }
    
    pub fn apply_theme(&self) {
        let css = self.theme.borrow().generate_css();
        self.css_provider.load_from_data(&css);

        let user_css_path = Theme::style_css_path();
        if let Some(ref path) = user_css_path {
            if path.exists() {
                self.user_css_provider.load_from_path(path);
            } else {
                self.user_css_provider.load_from_data("");
            }
        } else {
            self.user_css_provider.load_from_data("");
        }
    }
    
    pub fn show(&self) {
        self.window.set_visible(true);
        self.window.present();
        self.window.set_keyboard_mode(KeyboardMode::OnDemand);
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

    pub fn apply_position(&self) {
        // Reset all anchors and margins
        self.window.set_anchor(Edge::Top, false);
        self.window.set_anchor(Edge::Bottom, false);
        self.window.set_anchor(Edge::Left, false);
        self.window.set_anchor(Edge::Right, false);
        self.window.set_margin(Edge::Top, 0);
        self.window.set_margin(Edge::Bottom, 0);
        self.window.set_margin(Edge::Left, 0);
        self.window.set_margin(Edge::Right, 0);

        let config = self.config.borrow();
        let (col, row) = config.position_tuple();

        match (col, row) {
            (0, 0) => {
                self.window.set_anchor(Edge::Top, true);
                self.window.set_anchor(Edge::Left, true);
                self.window.set_margin(Edge::Top, config.margin_top);
                self.window.set_margin(Edge::Left, config.margin_left);
            }
            (1, 0) => {
                self.window.set_anchor(Edge::Top, true);
                self.window.set_margin(Edge::Top, config.margin_top);
            }
            (2, 0) => {
                self.window.set_anchor(Edge::Top, true);
                self.window.set_anchor(Edge::Right, true);
                self.window.set_margin(Edge::Top, config.margin_top);
                self.window.set_margin(Edge::Right, config.margin_right);
            }
            (0, 1) => {
                self.window.set_anchor(Edge::Left, true);
                self.window.set_margin(Edge::Left, config.margin_left);
            }
            (1, 1) => {}
            (2, 1) => {
                self.window.set_anchor(Edge::Right, true);
                self.window.set_margin(Edge::Right, config.margin_right);
            }
            (0, 2) => {
                self.window.set_anchor(Edge::Bottom, true);
                self.window.set_anchor(Edge::Left, true);
                self.window.set_margin(Edge::Bottom, config.margin_bottom);
                self.window.set_margin(Edge::Left, config.margin_left);
            }
            (1, 2) => {
                self.window.set_anchor(Edge::Bottom, true);
                self.window.set_margin(Edge::Bottom, config.margin_bottom);
            }
            (2, 2) => {
                self.window.set_anchor(Edge::Bottom, true);
                self.window.set_anchor(Edge::Right, true);
                self.window.set_margin(Edge::Bottom, config.margin_bottom);
                self.window.set_margin(Edge::Right, config.margin_right);
            }
            _ => {}
        }
    }

    pub fn set_position(&self, position: &str) {
        self.config.borrow_mut().position = position.to_string();
        self.apply_position();
    }

    pub fn reload_config(&self) {
        *self.config.borrow_mut() = Config::load();
        self.apply_position();
    }
    
    pub fn show_password_dialog<F: Fn(Option<String>) + 'static>(&self, ssid: &str, callback: F) {
        self.details_revealer.set_reveal_child(false);
        self.password_label.set_label(&format!("Enter password for {}:", ssid));
        self.password_entry.set_text("");
        self.password_error_label.set_label("");
        self.password_error_label.set_visible(false);
        self.password_connect_btn.set_label("Connect");
        self.password_connect_btn.set_sensitive(true);
        *self.password_callback.borrow_mut() = Some(Rc::new(callback));
        self.password_revealer.set_reveal_child(true);
        self.password_entry.grab_focus();
    }
    
    pub fn hide_password_dialog(&self) {
        self.password_entry.set_text("");
        self.password_error_label.set_label("");
        self.password_error_label.set_visible(false);
        self.password_connect_btn.set_label("Connect");
        self.password_connect_btn.set_sensitive(true);
        self.password_revealer.set_reveal_child(false);
        *self.password_callback.borrow_mut() = None;
    }

    pub fn show_hidden_dialog<F: Fn(Option<(String, String)>) + 'static>(&self, callback: F) {
        self.details_revealer.set_reveal_child(false);
        self.password_revealer.set_reveal_child(false);
        self.error_revealer.set_reveal_child(false);
        self.hidden_ssid_entry.set_text("");
        self.hidden_password_entry.set_text("");
        *self.hidden_callback.borrow_mut() = Some(Rc::new(callback));
        self.hidden_revealer.set_reveal_child(true);
        self.hidden_ssid_entry.grab_focus();
    }
    
    pub fn show_password_error(&self, message: &str) {
        let clean_msg = sanitize_error_message(message);
        self.password_error_label.set_label(&clean_msg);
        self.password_error_label.set_visible(true);
        self.password_connect_btn.set_label("Connect");
        self.password_connect_btn.set_sensitive(true);
        self.password_entry.grab_focus();
    }
    
    
    pub fn show_error(&self, message: &str) {
        // If password dialog is open, show error inline there instead
        if self.password_revealer.reveals_child() {
            self.show_password_error(message);
            return;
        }
        let clean_msg = sanitize_error_message(message);
        self.details_revealer.set_reveal_child(false);
        self.error_label.set_label(&clean_msg);
        self.error_revealer.set_reveal_child(true);
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

/// Sanitize D-Bus/system error messages into user-friendly text.
fn sanitize_error_message(message: &str) -> String {
    let msg_lower = message.to_lowercase();
    
    if msg_lower.contains("secret") || msg_lower.contains("password") 
        || msg_lower.contains("psk") || msg_lower.contains("802-11-wireless-security") {
        "Wrong password. Please try again.".to_string()
    } else if msg_lower.contains("no suitable") || msg_lower.contains("no network") {
        "Network not found. It may be out of range.".to_string()
    } else if msg_lower.contains("timeout") || msg_lower.contains("timed out") {
        "Connection timed out. Please try again.".to_string()
    } else if (msg_lower.contains("type") && msg_lower.contains("does not match"))
        || msg_lower.contains("a{sa{sv}}") {
        "Operation failed. Please try again.".to_string()
    } else if msg_lower.contains("rejected") || msg_lower.contains("auth") {
        "Authentication failed. Check your password.".to_string()
    } else if msg_lower.contains("not connected") || msg_lower.contains("not paired") {
        "Device is not connected.".to_string()
    } else if msg_lower.contains("already") && msg_lower.contains("connect") {
        "Already connected.".to_string()
    } else if msg_lower.contains("busy") || msg_lower.contains("in progress") {
        "Device is busy. Please wait and try again.".to_string()
    } else {
        // Fallback: try to extract a readable suffix after the last ": "
        if let Some(pos) = message.rfind(": ") {
            let suffix = &message[pos + 2..];
            if suffix.len() > 60 || suffix.contains('{') || suffix.contains('(') {
                "Operation failed. Please try again.".to_string()
            } else {
                suffix.to_string()
            }
        } else if message.len() > 80 {
            "Operation failed. Please try again.".to_string()
        } else {
            message.to_string()
        }
    }
}
