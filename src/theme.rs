use std::path::Path;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
struct ThemeFile {
    accent_primary: Option<String>,
    accent_secondary: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Theme {
    pub accent_primary: String,
    pub accent_secondary: String,
}

impl Default for Theme {
    fn default() -> Self {
        Theme {
            accent_primary: "#8B5CF6".to_string(),
            accent_secondary: "#D97706".to_string(),
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
                            return Theme {
                                accent_primary: theme_file.accent_primary
                                    .unwrap_or_else(|| Self::default().accent_primary),
                                accent_secondary: theme_file.accent_secondary
                                    .unwrap_or_else(|| Self::default().accent_secondary),
                            };
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
        let home = std::env::var("HOME").unwrap_or_else(|_| "/home".to_string());
        std::path::PathBuf::from(home)
            .join(".config")
            .join("orbit")
            .join("theme.toml")
    }
    
    pub fn generate_css(&self) -> String {
        let accent = &self.accent_primary;
        let gold = &self.accent_secondary;
        
        format!(r#"
/* ========================================
   ORBIT GLASSMORPHISM THEME
   Violet & Gold Color Scheme
   ======================================== */

/* Main Panel */
.orbit-panel {{
    background: rgba(15, 15, 20, 0.75);
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 16px;
    color: #ffffff;
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

/* Header - making it darker and using shades of violet/gold */
.orbit-header {{
    background: rgba(20, 15, 30, 0.9); /* Darker violet-tinted background */
    border-bottom: 1px solid rgba(139, 92, 246, 0.2); /* Subtle violet separator */
    border-radius: 16px 16px 0 0;
    margin: -8px -8px 8px -8px;
    padding: 16px 16px 8px 16px;
}}

/* Scan Button Footer - matching header contrast */
.orbit-footer {{
    background: rgba(20, 15, 30, 0.9); /* Darker violet-tinted background */
    border-top: 1px solid rgba(139, 92, 246, 0.2); /* Subtle violet separator */
    border-radius: 0 0 16px 16px;
    margin: 8px -8px -8px -8px;
    padding: 20px 24px;
}}

/* Contrast for items - using your gold accent */
.orbit-section-header {{
    color: {gold};
    font-weight: 800;
    text-shadow: 0 1px 2px rgba(0, 0, 0, 0.5);
}}

window {{
    background: none;
    background-color: transparent;
    box-shadow: none;
    border: none;
}}

/* Header - making it slightly darker/different to create contrast with panel */
.orbit-header {{
    background: rgba(0, 0, 0, 0.2);
    border-radius: 16px 16px 0 0;
    margin: -8px -8px 8px -8px;
    padding: 16px 16px 8px 16px;
}}

/* Scan Button Footer - also darker to match header contrast */
.orbit-footer {{
    background: rgba(0, 0, 0, 0.2);
    border-radius: 0 0 16px 16px;
    margin: 8px -8px -8px -8px;
    padding: 20px 24px;
}}

window {{
    background: transparent;
}}

/* Header */
.orbit-header {{
    background: transparent;
    padding: 16px 16px 8px 16px;
}}

/* Tabs - Unified pill, no separation */
.orbit-tab-bar {{
    background: rgba(255, 255, 255, 0.05);
    border-radius: 9999px;
    padding: 4px;
}}

.orbit-tab {{
    background: transparent;
    background-image: none;
    color: rgba(255, 255, 255, 0.5);
    border-radius: 0;
    padding: 8px 16px;
    border: none;
    box-shadow: none;
    outline: none;
    -gtk-icon-shadow: none;
    text-shadow: none;
    font-size: 12px;
    font-weight: 600;
    transition: all 0.25s cubic-bezier(0.4, 0, 0.2, 1);
    min-width: 80px;
}}

.orbit-tab:hover {{
    background: transparent;
    background-image: none;
    color: rgba(255, 255, 255, 0.85);
    box-shadow: none;
    border: none;
    outline: none;
}}

.orbit-tab:active {{
    background: transparent;
    background-image: none;
    outline: none;
}}

.orbit-tab:focus {{
    outline: none;
    box-shadow: none;
}}

.orbit-tab.active {{
    background: rgba(255, 255, 255, 0.15);
    border-radius: 9999px;
    color: #ffffff;
    box-shadow: 0 2px 10px rgba(0, 0, 0, 0.2);
}}

/* Glass Cards - Network/Device Rows */
.orbit-network-row,
.orbit-device-row,
.orbit-saved-network-row {{
    background: rgba(255, 255, 255, 0.08);
    border: 1px solid rgba(255, 255, 255, 0.05);
    border-radius: 12px;
    padding: 12px 14px;
    margin: 6px 8px;
    transition: all 0.2s ease;
}}

/* Card hover - Border glow + shadow tint */
.orbit-network-row:hover,
.orbit-device-row:hover,
.orbit-saved-network-row:hover {{
    background: rgba(255, 255, 255, 0.05);
    border-color: rgba(217, 119, 6, 0.35);
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.2), 0 0 0 1px rgba(217, 119, 6, 0.1);
}}

/* Connected/Active State - Violet to Gold gradient */
.orbit-network-row.connected,
.orbit-device-row.connected,
.orbit-saved-network-row.active {{
    background: linear-gradient(135deg, rgba(139, 92, 246, 0.15), rgba(217, 119, 6, 0.1));
    border: 1px solid rgba(139, 92, 246, 0.3);
}}

.orbit-network-row.connected:hover,
.orbit-device-row.connected:hover,
.orbit-saved-network-row.active:hover {{
    background: linear-gradient(135deg, rgba(139, 92, 246, 0.18), rgba(217, 119, 6, 0.12));
    border-color: rgba(139, 92, 246, 0.4);
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.2), 0 0 0 1px rgba(139, 92, 246, 0.15);
}}

/* Buttons - Universal Hard Reset */
button,
button:hover,
button:active,
button:focus,
button:disabled,
.orbit-button,
.orbit-button:hover,
.orbit-button:active,
.orbit-button:focus,
.orbit-tab,
.orbit-tab:hover,
.orbit-tab:active,
.orbit-tab:focus {{
    background-image: none !important;
    box-shadow: none !important;
    text-shadow: none !important;
    -gtk-icon-shadow: none !important;
    outline: none !important;
    border-image: none !important;
}}

.orbit-button {{
    background: rgba(255, 255, 255, 0.05);
    color: #ffffff;
    border: 1px solid rgba(255, 255, 255, 0.05);
    border-radius: 9999px;
    padding: 6px 14px;
    font-size: 11px;
    font-weight: 700;
    transition: all 0.2s cubic-bezier(0.4, 0, 0.2, 1);
}}

.orbit-button:hover {{
    background: rgba(217, 119, 6, 0.15);
    border-color: rgba(217, 119, 6, 0.35);
    color: #ffffff;
}}

.orbit-button:active {{
    background: rgba(217, 119, 6, 0.25);
    transition-duration: 0.05s;
}}

.orbit-button.primary {{
    background: {accent};
    border: 1px solid rgba(255, 255, 255, 0.1);
    color: #ffffff;
}}

.orbit-button.primary:hover {{
    background: color-mix(in srgb, {accent} 85%, white);
    border-color: rgba(255, 255, 255, 0.2);
}}

/* Tab bar items specifically */
.orbit-tab-bar button {{
    background: transparent;
    background-image: none;
    box-shadow: none;
    border: none;
}}

.orbit-tab {{
    background: transparent;
    color: rgba(255, 255, 255, 0.5);
    border-radius: 0;
    padding: 8px 16px;
    border: none;
    font-size: 12px;
    font-weight: 600;
    transition: all 0.25s cubic-bezier(0.4, 0, 0.2, 1);
    min-width: 80px;
}}

.orbit-tab:hover {{
    background: transparent !important;
    color: rgba(255, 255, 255, 0.85);
}}

.orbit-tab.active {{
    background: rgba(255, 255, 255, 0.15) !important;
    border-radius: 9999px;
    color: #ffffff;
}}

.orbit-action-btn {{
    padding: 6px 14px;
    font-size: 11px;
}}

/* Power Toggle - Gold when ON, Gray when OFF */
.orbit-toggle-switch {{
    background: rgba(100, 100, 100, 0.5);
    border-radius: 9999px;
}}

.orbit-toggle-switch:checked {{
    background: rgba(217, 119, 6, 0.9);
}}

.orbit-toggle-switch slider {{
    background: #ffffff;
    border-radius: 9999px;
}}

/* Section Headers - Gold */
.orbit-section-header {{
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 0.1em;
    color: {gold};
    font-weight: 700;
    padding: 0 12px;
    margin-top: 8px;
    margin-bottom: 8px;
}}

/* Typography */
.orbit-title {{
    font-size: 18px;
    font-weight: 700;
    color: #ffffff;
    letter-spacing: -0.01em;
}}

.orbit-ssid {{
    font-weight: 600;
    font-size: 14px;
    color: #f1f5f9;
}}

.orbit-status {{
    font-size: 11px;
    color: #64748b;
}}

.orbit-detail-label {{
    font-size: 11px;
    color: #64748b;
}}

.orbit-detail-value {{
    font-size: 13px;
    color: #f1f5f9;
    font-weight: 500;
}}

.orbit-detail-icon {{
    color: rgba(139, 92, 246, 0.6);
}}

/* Icon Styling */
.orbit-signal-icon {{
    color: #94a3b8;
}}

.orbit-icon-accent {{
    color: {accent};
}}

/* List Container */
.orbit-list {{
    background: transparent;
}}

.orbit-scrolled {{
    background: transparent;
}}

/* Placeholder */
.orbit-placeholder {{
    color: rgba(255, 255, 255, 0.3);
    padding: 40px;
    font-size: 13px;
}}

/* Overlay Panels - Slide up from bottom */
.orbit-details-overlay,
.orbit-password-overlay,
.orbit-error-overlay {{
    background: rgba(10, 10, 15, 0.98);
    border-radius: 16px;
    box-shadow: 0 -8px 32px rgba(0, 0, 0, 0.8), 0 0 48px rgba(0, 0, 0, 0.5);
    padding: 20px;
}}

.orbit-details-overlay {{
    border: 1px solid rgba(255, 255, 255, 0.1);
}}

.orbit-password-overlay {{
    border: 1px solid rgba(255, 255, 255, 0.1);
}}

.orbit-password-overlay entry {{
    background: rgba(255, 255, 255, 0.08);
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 8px;
    padding: 12px 16px;
    color: #ffffff;
    font-size: 13px;
}}

.orbit-password-overlay entry:focus {{
    border-color: {accent};
    box-shadow: 0 0 0 2px rgba(139, 92, 246, 0.2);
}}

.orbit-details-row {{
    padding: 10px 0;
    border-bottom: 1px solid rgba(255, 255, 255, 0.05);
}}

.orbit-details-row:last-child {{
    border-bottom: none;
}}

.orbit-error-overlay {{
    background: rgba(239, 68, 68, 0.2);
    border: 1px solid rgba(239, 68, 68, 0.4);
    padding: 16px 20px;
}}

.orbit-error-label {{
    color: #f87171;
    font-size: 13px;
}}

/* Scan Button Footer */
.orbit-footer {{
    background: linear-gradient(to top, rgba(15, 15, 20, 1), rgba(15, 15, 20, 0.95), transparent);
    padding: 20px 24px;
}}

/* Scrollbar */
.orbit-scrolled scrollbar {{
    background: transparent;
}}

.orbit-scrolled scrollbar slider {{
    background: rgba(255, 255, 255, 0.1);
    border-radius: 10px;
    min-width: 4px;
}}

/* Icon Container - Violet */
.orbit-icon-container {{
    border-radius: 9999px;
    background: rgba(139, 92, 246, 0.2);
    padding: 10px;
}}

.orbit-icon-container image {{
    -gtk-icon-size: 20px;
}}
"#,
            accent = accent,
            gold = gold
        )
    }
}
