use std::io::{Read, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use gtk4::glib;

const SOCKET_NAME: &str = "orbit.sock";

#[derive(Debug, Clone)]
pub enum DaemonCommand {
    Show,
    Hide,
    Toggle(Option<String>),
    ReloadTheme,
    ReloadConfig,
    Quit,
}

impl DaemonCommand {
    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        let s = String::from_utf8_lossy(bytes);
        if s.starts_with("show") {
            Some(Self::Show)
        } else if s.starts_with("hide") {
            Some(Self::Hide)
        } else if s.starts_with("reload-theme") {
            Some(Self::ReloadTheme)
        } else if s.starts_with("reload-config") {
            Some(Self::ReloadConfig)
        } else if s.starts_with("toggle") {
            let parts: Vec<&str> = s.split(':').collect();
            let pos = if parts.len() > 1 && !parts[1].is_empty() {
                Some(parts[1].to_string())
            } else {
                None
            };
            Some(Self::Toggle(pos))
        } else if s.starts_with("quit") {
            Some(Self::Quit)
        } else {
            None
        }
    }
    
    fn to_string(&self) -> String {
        match self {
            Self::Show => "show".to_string(),
            Self::Hide => "hide".to_string(),
            Self::ReloadTheme => "reload-theme".to_string(),
            Self::ReloadConfig => "reload-config".to_string(),
            Self::Toggle(pos) => {
                if let Some(p) = pos {
                    format!("toggle:{}", p)
                } else {
                    "toggle".to_string()
                }
            }
            Self::Quit => "quit".to_string(),
        }
    }
}

pub struct DaemonServer {
    listener: Option<UnixListener>,
}

impl DaemonServer {
    pub fn new() -> Result<Self, std::io::Error> {
        let runtime_dir = std::env::var("XDG_RUNTIME_DIR")
            .unwrap_or_else(|_| format!("/tmp/orbit-{}", std::process::id()));
        let socket_path = PathBuf::from(&runtime_dir).join(SOCKET_NAME);
        
        if socket_path.exists() {
            std::fs::remove_file(&socket_path)?;
        }
        
        let listener = UnixListener::bind(&socket_path)?;
        listener.set_nonblocking(true)?;
        
        Ok(Self {
            listener: Some(listener),
        })
    }
    
    pub fn run<F>(mut self, callback: F) 
    where
        F: Fn(DaemonCommand) + Send + 'static,
    {
        let listener = Arc::new(Mutex::new(self.listener.take().unwrap()));
        
        glib::spawn_future_local(async move {
            loop {
                if let Ok((mut stream, _)) = listener.lock().unwrap().accept() {
                    let mut buf = [0u8; 64];
                    if let Ok(n) = stream.read(&mut buf) {
                        if n > 0 {
                            if let Some(cmd) = DaemonCommand::from_bytes(&buf[..n]) {
                                callback(cmd);
                                let _ = stream.write_all(b"ok");
                            } else {
                                let _ = stream.write_all(b"unknown");
                            }
                        }
                    }
                }
                glib::timeout_future(std::time::Duration::from_millis(50)).await;
            }
        });
    }
}



pub struct DaemonClient;

impl DaemonClient {
    pub fn send_command(cmd: DaemonCommand) -> Result<String, std::io::Error> {
        let runtime_dir = std::env::var("XDG_RUNTIME_DIR")
            .unwrap_or_else(|_| "/tmp".to_string());
        let socket_path = PathBuf::from(&runtime_dir).join(SOCKET_NAME);
        
        let mut stream = UnixStream::connect(&socket_path)?;
        stream.write_all(cmd.to_string().as_bytes())?;
        stream.flush()?;
        
        let mut buf = [0u8; 32];
        let n = stream.read(&mut buf)?;
        Ok(String::from_utf8_lossy(&buf[..n]).to_string())
    }
    
    pub fn is_daemon_running() -> bool {
        let runtime_dir = std::env::var("XDG_RUNTIME_DIR")
            .unwrap_or_else(|_| "/tmp".to_string());
        let socket_path = PathBuf::from(&runtime_dir).join(SOCKET_NAME);
        
        if !socket_path.exists() {
            return false;
        }
        
        match UnixStream::connect(&socket_path) {
            Ok(_) => true,
            Err(_) => {
                let _ = std::fs::remove_file(&socket_path);
                false
            }
        }
    }
}
