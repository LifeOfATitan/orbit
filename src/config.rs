use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    #[serde(default = "default_position")]
    pub position: String,
    
    #[serde(default = "default_margin")]
    pub margin_top: i32,
    
    #[serde(default = "default_margin")]
    pub margin_right: i32,
    
    #[serde(default = "default_margin")]
    pub margin_bottom: i32,
    
    #[serde(default = "default_margin")]
    pub margin_left: i32,
}

fn default_position() -> String { "center".to_string() }
fn default_margin() -> i32 { 10 }

impl Default for Config {
    fn default() -> Self {
        Self {
            position: default_position(),
            margin_top: default_margin(),
            margin_right: default_margin(),
            margin_bottom: default_margin(),
            margin_left: default_margin(),
        }
    }
}

impl Config {
    pub fn load() -> Self {
        let config_path = match Self::config_path() {
            Some(p) => p,
            None => return Self::default(),
        };
        
        if config_path.exists() {
            match std::fs::read_to_string(&config_path) {
                Ok(content) => {
                    match toml::from_str(&content) {
                        Ok(config) => {
                            return config;
                        }
                        Err(e) => {
                            eprintln!("Failed to parse config: {}", e);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Failed to read config: {}", e);
                }
            }
        }
        
        Self::default()
    }
    
    pub fn config_path() -> Option<PathBuf> {
        let home = std::env::var("HOME").ok()?;
        Some(PathBuf::from(home)
            .join(".config")
            .join("orbit")
            .join("config.toml"))
    }
    
    pub fn position_tuple(&self) -> (i32, i32) {
        match self.position.as_str() {
            "top-left" => (0, 0),
            "top-center" => (1, 0),
            "top-right" => (2, 0),
            "center-left" => (0, 1),
            "center" => (1, 1),
            "center-right" => (2, 1),
            "bottom-left" => (0, 2),
            "bottom-center" => (1, 2),
            "bottom-right" => (2, 2),
            _ => (1, 1),
        }
    }
}
