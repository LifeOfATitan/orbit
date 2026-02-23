use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
struct ThemeFile {
    accent_primary: Option<String>,
    accent_secondary: Option<String>,
    background: Option<String>,
    foreground: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Theme {
    pub accent_primary: String,
    pub accent_secondary: String,
    pub background: String,
    pub foreground: String,
}

impl Default for Theme {
    fn default() -> Self {
        Theme {
            accent_primary: "#8b5cf6".to_string(),
            accent_secondary: "#06b6d4".to_string(),
            background: "#1e1e2e".to_string(),
            foreground: "#d4d4d8".to_string(),
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

        let separator = self.hex_to_rgba(accent, 0.3);
        
        format!(r#"
/* ========================================
   ORBIT AGGRESSIVE RESET & THEME
   ======================================== */

/* Nuke all default shadows and focus rings */
* {{
    box-shadow: none !important;
    outline: none !important;
    -gtk-outline-radius: inherit;
    background-image: none !important;
}}

/* Main Panel */
.orbit-panel {{
    background-color: {panel_bg} !important;
    background-image: none !important;
    border: 1px solid rgba(255, 255, 255, 0.15) !important;
    border-radius: 16px;
    color: {fg};
    padding: 8px;
    margin: 0;
    background-clip: padding-box;
}}

window {{
    background: none !important;
    background-color: transparent !important;
    box-shadow: none !important;
    border: none !important;
}}

.background {{
    background-color: transparent !important;
    background-image: none !important;
    box-shadow: none !important;
}}

/* Header */
.orbit-header {{
    background-color: {section_bg} !important;
    background-image: none !important;
    border-bottom: 1px solid {separator} !important;
    border-radius: 16px 16px 0 0;
    margin: -8px -8px 8px -8px;
    padding: 16px 16px 12px 16px;
    background-clip: padding-box;
}}

.orbit-tab-bar {{
    background-color: rgba(255, 255, 255, 0.05) !important;
    border-radius: 9999px !important;
    padding: 4px !important;
    margin-top: 8px !important;
    background-image: none !important;
    box-shadow: none !important;
}}

.orbit-tab-bar button {{
    background-image: none !important;
    background-color: transparent !important;
    border: none !important;
    box-shadow: none !important;
    border-radius: 9999px !important;
    margin: 0 !important;
    padding: 0 !important;
    min-height: 32px !important;
    min-width: 80px !important;
}}

.orbit-tab-bar button label {{
    padding: 8px 16px !important;
    margin: 0 !important;
    color: {fg} !important;
    font-weight: 700 !important;
    background-image: none !important;
    background-color: transparent !important;
    border-radius: 9999px !important;
    transition: all 0.2s ease !important;
    box-shadow: none !important;
    border: none !important;
    -gtk-icon-shadow: none !important;
    text-shadow: none !important;
}}


.orbit-tab-bar button:hover label,
.orbit-tab-bar button.active label {{
    background-color: {accent} !important;
    color: #ffffff !important;
    background-image: none !important;
    box-shadow: none !important;
    border-radius: 9999px !important;
}}


.orbit-tab-bar button:hover label {{
    background-color: {accent} !important;
    color: #ffffff !important;
    background-image: none !important;
    box-shadow: none !important;
}}

.orbit-tab-bar button.active label {{
    background-color: {accent} !important;
    color: #ffffff !important;
    background-image: none !important;
    box-shadow: none !important;
}}


.orbit-tab-bar button:hover label,
.orbit-tab-bar button.active label {{
    background-color: {accent} !important;
    color: #ffffff !important;
    background-image: none !important;
    box-shadow: none !important;
    border: none !important;
    border-radius: 9999px !important;
}}


.orbit-tab:hover {{
    opacity: 1.0 !important;
    color: #ffffff !important;
    background-color: {accent} !important;
    background-image: none !important;
}}

.orbit-tab.active {{
    background-color: {accent} !important;
    background-image: none !important;
    border-radius: 9999px;
    color: #ffffff !important;
    opacity: 1.0 !important;
    box-shadow: none !important;
}}

/* Glass Cards */
.orbit-network-row,
.orbit-device-row,
.orbit-saved-network-row {{
    background-color: {card_bg} !important;
    background-image: none !important;
    border: 1px solid rgba(255, 255, 255, 0.05) !important;
    border-radius: 12px !important;
    padding: 12px 14px !important;
    margin: 4px 8px !important;
    transition: all 0.2s ease !important;
    background-clip: padding-box !important;
    box-shadow: none !important;
}}

.orbit-network-row:hover,
.orbit-device-row:hover,
.orbit-saved-network-row:hover {{
    background-color: rgba(255, 255, 255, 0.1) !important;
    border-color: {accent} !important;
    box-shadow: none !important;
    transform: none !important;
}}

/* Connected State */
.orbit-network-row.connected,
.orbit-device-row.connected,
.orbit-saved-network-row.active {{
    background-color: {accent} !important;
    background-image: none !important;
    border: 1px solid {accent} !important;
    color: #ffffff !important;
    box-shadow: none !important;
}}

.orbit-network-row.connected label,
.orbit-device-row.connected label,
.orbit-saved-network-row.active label {{
    color: #ffffff !important;
}}


.orbit-network-row:hover,
.orbit-device-row:hover,
.orbit-saved-network-row:hover {{
    border-color: {accent} !important;
    background-color: rgba(255, 255, 255, 0.12) !important;
    background-image: none !important;
}}

.orbit-network-row.connected,
.orbit-device-row.connected,
.orbit-saved-network-row.active {{
    background-color: {accent} !important;
    background-image: none !important;
    border: 1px solid {accent} !important;
    border-radius: 12px !important;
    background-clip: padding-box !important;
    box-shadow: none !important;
}}

.orbit-network-row.connected label,
.orbit-device-row.connected label,
.orbit-saved-network-row.active label {{
    color: #ffffff !important;
}}


.orbit-network-row.connected label,
.orbit-device-row.connected label,
.orbit-saved-network-row.active label {{
    color: #ffffff !important;
}}

/* Buttons */
.orbit-button {{
    background-color: rgba(255, 255, 255, 0.08) !important;
    background-image: none !important;
    color: {fg} !important;
    border: 1px solid rgba(255, 255, 255, 0.1) !important;
    border-radius: 9999px;
    padding: 8px 18px;
    font-size: 10px;
    font-weight: 700;
    transition: all 0.15s ease;
    background-clip: padding-box;
    box-shadow: none !important;
}}

.orbit-button:hover,
.orbit-network-row .orbit-button:hover,
.orbit-device-row .orbit-button:hover {{
    background-color: {accent} !important;
    border-color: {accent} !important;
    background-image: none !important;
    color: #ffffff !important;
    box-shadow: none !important;
}}

.orbit-button.primary,
.orbit-network-row .orbit-button.primary,
.orbit-device-row .orbit-button.primary {{
    background-color: {accent} !important;
    background-image: none !important;
    color: #ffffff !important;
    border: none !important;
}}


.orbit-button.primary {{
    background-color: {accent} !important;
    background-image: none !important;
    border: none !important;
    color: #ffffff !important;
}}

.orbit-button.primary:hover {{
    filter: brightness(1.1) !important;
}}

.orbit-button.destructive:hover {{
    background-color: #ef4444 !important;
    border-color: #ef4444 !important;
}}

/* Overlays */
.orbit-details-overlay, 
.orbit-password-overlay, 
.orbit-error-overlay {{
    background-color: {opaque_bg} !important;
    border: 2px solid {accent} !important;
    border-radius: 16px;
    color: {fg} !important;
    margin: 20px;
    padding: 24px;
    background-clip: padding-box;
}}

/* Section Headers */
.orbit-section-header {{
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 0.15em;
    color: {gold} !important;
    font-weight: 800;
    padding: 0 12px;
    margin: 12px 0 8px 0;
}}

.orbit-ssid {{
    font-weight: 700;
    font-size: 14px;
}}

/* Inputs */
entry, password-entry {{
    background-color: rgba(255, 255, 255, 0.05) !important;
    background-image: none !important;
    border: 1px solid rgba(255, 255, 255, 0.1) !important;
    color: {fg} !important;
    border-radius: 12px;
    padding: 10px 14px;
    background-clip: padding-box;
}}

entry:focus, password-entry:focus {{
    border-color: {accent} !important;
}}
"#,
            panel_bg = panel_bg,
            section_bg = section_bg,
            opaque_bg = opaque_bg,
            card_bg = card_bg,
            separator = separator,
            accent = accent,
            gold = gold,
            fg = fg
        )
    }
}
