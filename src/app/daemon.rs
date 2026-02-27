use std::io::{Read, Write};
use std::os::unix::net::{UnixStream as StdUnixStream};
use std::path::PathBuf;
use tokio::net::UnixListener;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

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

fn get_socket_path() -> PathBuf {
    let path = std::env::var("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let user = std::env::var("USER").unwrap_or_else(|_| "user".to_string());
            PathBuf::from(format!("/tmp/orbit-{}", user))
        })
        .join(SOCKET_NAME);
    
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    
    path
}

pub struct DaemonServer {
    listener: Option<UnixListener>,
    path: PathBuf,
}

impl DaemonServer {
    pub async fn new() -> Result<Self, std::io::Error> {
        let socket_path = get_socket_path();
        log::info!("Starting daemon on socket: {:?}", socket_path);
        
        if socket_path.exists() {
            if let Ok(_) = StdUnixStream::connect(&socket_path) {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::AddrInUse,
                    "Daemon is already running",
                ));
            }
            let _ = std::fs::remove_file(&socket_path);
        }
        
        let listener = UnixListener::bind(&socket_path)?;
        
        Ok(Self {
            listener: Some(listener),
            path: socket_path,
        })
    }
    
    pub fn run<F>(mut self, callback: F) 
    where
        F: Fn(DaemonCommand) + Send + 'static,
    {
        if let Some(listener) = self.listener.take() {
            // Use a dedicated thread with its own tokio runtime to ensure the listener 
            // is never blocked by the GTK main loop and stays alive.
            std::thread::spawn(move || {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .unwrap();
                
                let _server_guard = self;
                
                rt.block_on(async {
                    loop {
                        match listener.accept().await {
                            Ok((mut stream, _)) => {
                                let mut buf = [0u8; 64];
                                match stream.read(&mut buf).await {
                                    Ok(n) if n > 0 => {
                                        if let Some(cmd) = DaemonCommand::from_bytes(&buf[..n]) {
                                            callback(cmd);
                                            // Ensure the write completes before closing
                                            let _ = stream.write_all(b"ok").await;
                                            let _ = stream.flush().await;
                                        } else {
                                            let _ = stream.write_all(b"unknown").await;
                                        }
                                    }
                                    _ => {}
                                }
                            }
                            Err(e) => {
                                log::error!("Socket accept error: {}", e);
                            }
                        }
                    }
                });
            });
        }
    }
}

impl Drop for DaemonServer {
    fn drop(&mut self) {
        if self.path.exists() {
            log::info!("Cleaning up socket: {:?}", self.path);
            let _ = std::fs::remove_file(&self.path);
        }
    }
}

pub struct DaemonClient;

impl DaemonClient {
    pub fn send_command(cmd: DaemonCommand) -> Result<String, std::io::Error> {
        let socket_path = get_socket_path();
        
        let mut stream = StdUnixStream::connect(&socket_path)?;
        // Set a timeout so we don't hang if the server is unresponsive
        stream.set_read_timeout(Some(std::time::Duration::from_secs(2)))?;
        stream.set_write_timeout(Some(std::time::Duration::from_secs(2)))?;
        
        stream.write_all(cmd.to_string().as_bytes())?;
        stream.flush()?;
        
        let mut buf = [0u8; 32];
        let n = stream.read(&mut buf)?;
        Ok(String::from_utf8_lossy(&buf[..n]).to_string())
    }
    
    pub fn is_daemon_running() -> bool {
        let socket_path = get_socket_path();
        
        if !socket_path.exists() {
            return false;
        }
        
        match StdUnixStream::connect(&socket_path) {
            Ok(_) => true,
            Err(_) => {
                let _ = std::fs::remove_file(&socket_path);
                false
            }
        }
    }
}
