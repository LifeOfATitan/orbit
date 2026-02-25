use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
struct ThemeFile {
    accent_primary: Option<String>,
    accent_secondary: Option<String>,
    background: Option<String>,
    foreground: Option<String>,
    destructive: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Theme {
    pub accent_primary: String,
    pub accent_secondary: String,
    pub background: String,
    pub foreground: String,
    pub destructive: String,
}

impl Default for Theme {
    fn default() -> Self {
        Theme {
            accent_primary: "#8b5cf6".to_string(),
            accent_secondary: "#06b6d4".to_string(),
            background: "#1e1e2e".to_string(),
            foreground: "#d4d4d8".to_string(),
            destructive: "#ef4444".to_string(),
        }
    }
}

impl Theme {
    pub fn load() -> Self {
        let theme_path = Self::theme_path();
        
        if theme_path.exists() {
            match std::fs::read_to_string(&theme_path) {
                Ok(content) => {
                    match toml::from_str::<ThemeFile>(&content) {
                        Ok(theme_file) => {
                            let mut theme = Self::default();
                            if let Some(c) = theme_file.accent_primary { theme.accent_primary = c; }
                            if let Some(c) = theme_file.accent_secondary { theme.accent_secondary = c; }
                            if let Some(c) = theme_file.background { theme.background = c; }
                            if let Some(c) = theme_file.foreground { theme.foreground = c; }
                            if let Some(c) = theme_file.destructive { theme.destructive = c; }
                            return theme;
                        }
                        Err(e) => {
                            eprintln!("Failed to parse theme file: {}", e);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Failed to read theme file: {}", e);
                }
            }
        }
        
        Self::default()
    }
    
    pub fn theme_path() -> std::path::PathBuf {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/home/amadeus".to_string());
        std::path::PathBuf::from(home)
            .join(".config")
            .join("orbit")
            .join("theme.toml")
    }

    pub fn style_css_path() -> std::path::PathBuf {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/home/amadeus".to_string());
        std::path::PathBuf::from(home)
            .join(".config")
            .join("orbit")
            .join("style.css")
    }

    fn hex_to_rgb(&self, hex: &str) -> (u8, u8, u8) {
        let hex = hex.trim_start_matches('#');
        if hex.len() != 6 {
            return (0, 0, 0);
        }
        let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0);
        let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0);
        let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0);
        (r, g, b)
    }

    fn get_luminance(&self, hex: &str) -> f32 {
        let (r, g, b) = self.hex_to_rgb(hex);
        (0.299 * r as f32 + 0.587 * g as f32 + 0.114 * b as f32) / 255.0
    }

    fn adjust_color(&self, hex: &str, factor: f32) -> String {
        let (r, g, b) = self.hex_to_rgb(hex);
        let is_light = self.get_luminance(hex) > 0.5;
        
        let new_factor = if is_light { 1.0 - factor } else { 1.0 + factor };
        
        let nr = (r as f32 * new_factor).clamp(0.0, 255.0) as u8;
        let ng = (g as f32 * new_factor).clamp(0.0, 255.0) as u8;
        let nb = (b as f32 * new_factor).clamp(0.0, 255.0) as u8;
        
        format!("#{:02x}{:02x}{:02x}", nr, ng, nb)
    }

    fn hex_to_rgba(&self, hex: &str, alpha: f32) -> String {
        let (r, g, b) = self.hex_to_rgb(hex);
        format!("rgba({}, {}, {}, {})", r, g, b, alpha)
    }
    
    pub fn generate_css(&self) -> String {
        let accent = &self.accent_primary;
        let gold = &self.accent_secondary;
        let bg = &self.background;
        let fg = &self.foreground;
        
        let is_dark = self.get_luminance(bg) < 0.5;
        
        let section_bg_hex = self.adjust_color(bg, 0.2); 
        let panel_bg = self.hex_to_rgba(bg, 0.91);
        let section_bg = self.hex_to_rgba(&section_bg_hex, 0.94);
        let opaque_bg = self.hex_to_rgba(bg, 0.99); 
        
        let card_bg = if is_dark {
            "rgba(255, 255, 255, 0.08)".to_string()
        } else {
            "rgba(0, 0, 0, 0.04)".to_string()
        };

        let card_hover_bg = if is_dark {
            "rgba(255, 255, 255, 0.15)".to_string()
        } else {
            "rgba(0, 0, 0, 0.1)".to_string()
        };

        let separator = self.hex_to_rgba(accent, 0.3);
        let destructive = &self.destructive;
        let destructive_separator = self.hex_to_rgba(destructive, 0.3);
        let connected_hover_separator = self.hex_to_rgba(accent, 0.4);
        let accent_hover = self.adjust_color(accent, 0.15);
        let destructive_hover = self.adjust_color(destructive, 0.15);
        
        format!(r#"
/* ========================================
   ORBIT DYNAMIC THEME
   ======================================== */

/* Main Panel */
.orbit-panel {{
    background-color: {panel_bg};
    background-image: linear-gradient(to bottom, rgba(255, 255, 255, 0.05), transparent);
    border: 1px solid rgba(255, 255, 255, 0.15);
    border-radius: 16px;
    color: {fg};
    padding: 8px;
    margin: 0;
}}

window {{
    background: none;
    background-color: transparent;
    box-shadow: none;
    border: none;
    border-radius: 16px;
}}

.background {{
    background-color: transparent;
    background-image: none;
    border-radius: 16px;
}}

/* Header */
.orbit-header {{
    background-color: {section_bg};
    background-image: linear-gradient(to bottom, rgba(255, 255, 255, 0.03), transparent);
    border-bottom: 1px solid {separator};
    border-radius: 16px 16px 0 0;
    margin: -8px -8px 8px -8px;
    padding: 16px 16px 8px 16px;
}}

/* Tabs */
.orbit-tab-bar {{
    background-color: rgba(255, 255, 255, 0.05);
    border-radius: 9999px;
    padding: 4px;
}}

.orbit-tab {{
    background: transparent;
    background-image: none;
    color: {fg};
    opacity: 0.6;
    border: none;
    box-shadow: none;
    outline: none;
    font-size: 11px;
    font-weight: 600;
    -gtk-icon-shadow: none;
    text-shadow: none;
    transition: background-color 0.2s ease, color 0.2s ease, opacity 0.2s ease, box-shadow 0.2s ease;
    min-width: 80px;
}}

.orbit-tab:hover {{
    opacity: 1.0;
    color: #ffffff;
    background-color: {accent};
    background-image: none;
    border-radius: 9999px;
    box-shadow: none;
}}

.orbit-tab.active {{
    background-color: {accent};
    background-image: none;
    border-radius: 9999px;
    color: #ffffff;
    opacity: 1.0;
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.2);
}}

/* Overlays - Opaque with padding */
.orbit-details-overlay, 
.orbit-password-overlay, 
.orbit-error-overlay {{
    background-color: {opaque_bg};
    border: 2px solid {accent};
    border-radius: 16px;
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.6);
    color: {fg};
    margin: 20px;
    padding: 24px;
}}

.orbit-details-overlay label,
.orbit-password-overlay label {{
    color: {fg};
}}

/* Glass Cards */
.orbit-network-row,
.orbit-device-row,
.orbit-saved-network-row {{
    background-color: {card_bg};
    border: 1px solid rgba(255, 255, 255, 0.05);
    border-radius: 12px;
    padding: 12px 14px;
    margin: 6px 8px;
    transition: background-color 0.25s cubic-bezier(0.4, 0, 0.2, 1), border-color 0.25s cubic-bezier(0.4, 0, 0.2, 1), box-shadow 0.25s cubic-bezier(0.4, 0, 0.2, 1);
}}

.orbit-network-row:hover,
.orbit-device-row:hover,
.orbit-saved-network-row:hover {{
    background-color: {card_hover_bg};
    background-image: none;
    border-color: {accent};
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.4);
}}

/* Connected State */
.orbit-network-row.connected,
.orbit-device-row.connected,
.orbit-saved-network-row.active {{
    background: linear-gradient(135deg, {separator}, rgba(0,0,0,0.15));
    border: 1px solid {accent};
    box-shadow: 0 0 10px {separator};
}}

/* Connected/Active Hover State */
.orbit-network-row.connected:hover,
.orbit-device-row.connected:hover,
.orbit-saved-network-row.active:hover {{
    background: linear-gradient(135deg, {connected_hover_separator}, rgba(0,0,0,0.1));
    border-color: {accent};
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.4), 0 0 12px {separator};
}}

/* Keyboard Focus State */
.orbit-network-row.focused,
.orbit-device-row.focused,
.orbit-saved-network-row.focused {{
    border-color: {accent};
    box-shadow: 0 0 0 2px {separator};
}}

/* Buttons */
.orbit-button {{
    background-color: rgba(255, 255, 255, 0.08);
    background-image: none;
    color: {fg};
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 9999px;
    padding: 8px 18px;
    font-size: 10px;
    font-weight: 700;
    box-shadow: none;
    outline: none;
    min-height: 0;
    min-width: 0;
    -gtk-icon-shadow: none;
    text-shadow: none;
    transition: background-color 0.2s ease, border-color 0.2s ease, color 0.2s ease, box-shadow 0.2s ease;
}}

.orbit-button:hover {{
    background-color: {accent};
    background-image: none;
    border-color: {accent};
    color: #ffffff;
    box-shadow: 0 0 12px rgba(0, 0, 0, 0.3);
    padding: 8px 18px;
    min-height: 0;
    min-width: 0;
    outline: none;
}}

.orbit-button.primary {{
    background-color: {accent};
    background-image: none;
    color: #ffffff;
    box-shadow: 0 4px 12px {separator};
    border: 1px solid transparent;
}}

.orbit-button.primary:hover {{
    background-color: {accent_hover};
    background-image: none;
    color: #ffffff;
    box-shadow: 0 6px 16px {separator};
    padding: 8px 18px;
    min-height: 0;
    min-width: 0;
    outline: none;
}}

/* Destructive Buttons */
.orbit-button.destructive {{
    background-color: {destructive};
    background-image: none;
    color: #ffffff;
    border: 1px solid transparent;
    box-shadow: 0 4px 12px {destructive_separator};
}}

.orbit-button.destructive:hover {{
    background-color: {destructive_hover};
    background-image: none;
    color: #ffffff;
    box-shadow: 0 6px 16px {destructive_separator};
    padding: 8px 18px;
    min-height: 0;
    min-width: 0;
    outline: none;
}}

/* Section Headers */
.orbit-section-header {{
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 0.15em;
    color: {gold};
    font-weight: 800;
    padding: 0 12px;
    margin-top: 12px;
    margin-bottom: 8px;
    text-shadow: 0 1px 2px rgba(0, 0, 0, 0.5);
}}

/* Footer */
.orbit-footer {{
    background-color: {section_bg};
    border-top: 1px solid {separator};
    border-radius: 0 0 16px 16px;
    margin: 8px -8px -8px -8px;
    padding: 24px 28px;
}}

.orbit-ssid {{
    font-weight: 700;
    font-size: 14px;
    color: {fg};
}}

.orbit-detail-label {{
    font-size: 10px;
    color: {fg};
    opacity: 0.7;
}}

.orbit-detail-value {{
    font-size: 12px;
    color: {fg};
    font-weight: 600;
}}

.orbit-icon-accent {{
    color: {accent};
}}

/* Inputs */
entry, password-entry {{
    background-color: rgba(255, 255, 255, 0.05);
    border: 1px solid rgba(255, 255, 255, 0.1);
    color: {fg};
    border-radius: 12px;
    padding: 10px 14px;
    min-height: 20px;
}}

password-entry > text {{
    margin-left: 8px;
    margin-right: 8px;
}}

entry:focus, password-entry:focus {{
    border-color: {accent};
    box-shadow: 0 0 0 1px {accent};
}}

/* Password dialog error */
.orbit-password-error {{
    color: {destructive};
    font-size: 12px;
    font-weight: 500;
}}

/* Connecting state */
.orbit-button.connecting {{
    opacity: 0.7;
}}

/* Row error state */
.orbit-status-error {{
    color: {destructive};
    font-size: 11px;
}}

/* Placeholder text (scanning, empty states) */
.orbit-placeholder {{
    color: {fg};
    opacity: 0.5;
    font-size: 13px;
    font-style: italic;
    padding: 32px 16px;
}}

/* Status line (under device/network name) */
.orbit-status {{
    font-size: 11px;
    color: {fg};
    opacity: 0.6;
}}

/* Signal/device type icon (disconnected state) */
.orbit-signal-icon {{
    color: {fg};
    opacity: 0.5;
}}

/* Signal strength bars */
.orbit-signal-bar-active {{
    background-color: {fg};
    opacity: 0.7;
    border-radius: 1px;
}}

.orbit-signal-bar-active-accent {{
    background-color: {accent};
    border-radius: 1px;
}}

.orbit-signal-bar-inactive {{
    background-color: {fg};
    opacity: 0.15;
    border-radius: 1px;
}}

.orbit-signal-bars-pad {{
    padding: 2px;
}}

/* Icon container (connected state badge) */
.orbit-icon-container {{
    background-color: {separator};
    border-radius: 8px;
    padding: 6px;
}}

/* Power toggle switch */
.orbit-toggle-switch {{
    background-color: rgba(255, 255, 255, 0.12);
    border-radius: 9999px;
}}

.orbit-toggle-switch:checked {{
    background-color: {accent};
}}
"#,
            panel_bg = panel_bg,
            section_bg = section_bg,
            opaque_bg = opaque_bg,
            card_bg = card_bg,
            card_hover_bg = card_hover_bg,
            separator = separator,
            connected_hover_separator = connected_hover_separator,
            accent = accent,
            accent_hover = accent_hover,
            gold = gold,
            fg = fg,
            destructive = destructive,
            destructive_separator = destructive_separator,
            destructive_hover = destructive_hover
        )
    }
}
