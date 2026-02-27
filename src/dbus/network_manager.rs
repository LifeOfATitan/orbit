use zbus::Connection;
use std::collections::HashMap;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AccessPoint {
    pub ssid: String,
    pub signal_strength: u8,
    pub security: SecurityType,
    pub is_connected: bool,
    pub device_path: String,
    pub path: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SavedNetwork {
    pub ssid: String,
    pub path: String,
    pub autoconnect: bool,
    pub is_active: bool,
}
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct VpnConnection {
    pub name: String,
    uuid: String,
    pub path: String,
    pub connection_type: String,
    pub is_active: bool,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct NetworkDetails {
    pub ssid: String,
    pub ip4_address: String,
    pub gateway: String,
    pub dns_servers: Vec<String>,
    pub mac_address: String,
    pub connection_speed: String,
    pub is_connected: bool,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum SecurityType {
    None,
    WEP,
    WPA,
    WPA2,
    WPA3,
}

#[derive(Clone)]
pub struct NetworkManager {
    conn: Connection,
}

impl NetworkManager {
    pub async fn new() -> zbus::Result<Self> {
        let conn = Connection::system().await?;
        Ok(Self { conn })
    }
    
    pub async fn is_wifi_enabled(&self) -> zbus::Result<bool> {
        let reply = self.conn
            .call_method(
                Some("org.freedesktop.NetworkManager"),
                "/org/freedesktop/NetworkManager",
                Some("org.freedesktop.DBus.Properties"),
                "Get",
                &("org.freedesktop.NetworkManager", "WirelessEnabled"),
            )
            .await?
            .body()
            .deserialize::<zbus::zvariant::OwnedValue>()?;
        
        bool::try_from(reply).map_err(zbus::Error::from)
    }
    
    pub async fn set_wifi_enabled(&self, enabled: bool) -> zbus::Result<()> {
        let value = zbus::zvariant::Value::Bool(enabled);
        self.conn
            .call_method(
                Some("org.freedesktop.NetworkManager"),
                "/org/freedesktop/NetworkManager",
                Some("org.freedesktop.DBus.Properties"),
                "Set",
                &("org.freedesktop.NetworkManager", "WirelessEnabled", value),
            )
            .await?;
        Ok(())
    }
    
    pub async fn check_connectivity(&self) -> zbus::Result<u32> {
        let reply = self.conn
            .call_method(
                Some("org.freedesktop.NetworkManager"),
                "/org/freedesktop/NetworkManager",
                Some("org.freedesktop.NetworkManager"),
                "CheckConnectivity",
                &(),
            )
            .await?
            .body()
            .deserialize::<u32>()?;
        Ok(reply)
    }
    
    pub async fn scan(&self) -> zbus::Result<()> {
        let devices = self.get_wireless_devices().await?;
        
        for device_path in devices {
            let path: zbus::zvariant::ObjectPath = device_path.as_str().try_into()
                .map_err(|e: zbus::zvariant::Error| zbus::Error::Variant(e))?;
            self.conn
                .call_method(
                    Some("org.freedesktop.NetworkManager"),
                    &path,
                    Some("org.freedesktop.NetworkManager.Device.Wireless"),
                    "RequestScan",
                    &HashMap::<String, zbus::zvariant::Value>::new(),
                )
                .await?;
        }
        
        Ok(())
    }
    
    pub async fn get_wifi_device_state(&self) -> zbus::Result<u32> {
        let devices = self.get_wireless_devices().await?;
        if let Some(device_path) = devices.get(0) {
            let path: zbus::zvariant::ObjectPath = device_path.as_str().try_into()
                .map_err(|e: zbus::zvariant::Error| zbus::Error::Variant(e))?;
            let reply = self.conn
                .call_method(
                    Some("org.freedesktop.NetworkManager"),
                    &path,
                    Some("org.freedesktop.DBus.Properties"),
                    "Get",
                    &("org.freedesktop.NetworkManager.Device", "State"),
                )
                .await?
                .body()
                .deserialize::<zbus::zvariant::OwnedValue>()?;
            
            let state: u32 = match u32::try_from(zbus::zvariant::Value::from(reply)) {
                Ok(t) => t,
                Err(_) => 0,
            };
            return Ok(state);
        }
        Ok(0)
    }

    pub async fn get_wireless_devices(&self) -> zbus::Result<Vec<String>> {
        let devices: Vec<zbus::zvariant::OwnedObjectPath> = self.conn
            .call_method(
                Some("org.freedesktop.NetworkManager"),
                "/org/freedesktop/NetworkManager",
                Some("org.freedesktop.NetworkManager"),
                "GetDevices",
                &(),
            )
            .await?
            .body()
            .deserialize()?;
        
        let mut wireless = Vec::new();
        
        for device_path in devices {
            let dtype_reply = self.conn
                .call_method(
                    Some("org.freedesktop.NetworkManager"),
                    &device_path,
                    Some("org.freedesktop.DBus.Properties"),
                    "Get",
                    &("org.freedesktop.NetworkManager.Device", "DeviceType"),
                )
                .await?
                .body()
                .deserialize::<zbus::zvariant::OwnedValue>()?;
            
            let device_type: u32 = match u32::try_from(zbus::zvariant::Value::from(dtype_reply)) {
                Ok(t) => t,
                Err(_) => 0,
            };
            
            if device_type == 2 {
                wireless.push(device_path.to_string());
            }
        }
        
        Ok(wireless)
    }
    
    pub async fn get_access_points(&self) -> zbus::Result<Vec<AccessPoint>> {
        let devices = self.get_wireless_devices().await?;
        let mut access_points = Vec::new();
        let active_ssid = self.get_active_ssid().await;
        
        for device_path in devices {
            let path: zbus::zvariant::ObjectPath = device_path.as_str().try_into()
                .map_err(|e: zbus::zvariant::Error| zbus::Error::Variant(e))?;
            let ap_paths: Vec<zbus::zvariant::OwnedObjectPath> = self.conn
                .call_method(
                    Some("org.freedesktop.NetworkManager"),
                    &path,
                    Some("org.freedesktop.NetworkManager.Device.Wireless"),
                    "GetAllAccessPoints",
                    &(),
                )
                .await?
                .body()
                .deserialize()?;
            
            for ap_path in ap_paths {
                if ap_path.as_str() == "/" {
                    continue;
                }
                
                let ssid_owned = self.get_ap_property(ap_path.as_str(), "Ssid").await;
                let ssid_bytes: Vec<u8> = ssid_owned
                    .ok()
                    .and_then(|ov| {
                        let v: zbus::zvariant::Value = ov.into();
                        if let zbus::zvariant::Value::Array(a) = v {
                            Some(a.iter().filter_map(|iv| {
                                u8::try_from(iv).ok()
                            }).collect())
                        } else {
                            None
                        }
                    })
                    .unwrap_or_default();
                
                let ssid = String::from_utf8_lossy(&ssid_bytes).to_string();
                
                if ssid.is_empty() {
                    continue;
                }
                
                let strength: u8 = self.get_ap_property(ap_path.as_str(), "Strength").await
                    .ok()
                    .and_then(|ov| u8::try_from(zbus::zvariant::Value::from(ov)).ok())
                    .unwrap_or(0);
                let flags: u32 = self.get_ap_property(ap_path.as_str(), "Flags").await
                    .ok()
                    .and_then(|ov| u32::try_from(zbus::zvariant::Value::from(ov)).ok())
                    .unwrap_or(0);
                let rsn_flags: u32 = self.get_ap_property(ap_path.as_str(), "RsnFlags").await
                    .ok()
                    .and_then(|ov| u32::try_from(zbus::zvariant::Value::from(ov)).ok())
                    .unwrap_or(0);
                let wpa_flags: u32 = self.get_ap_property(ap_path.as_str(), "WpaFlags").await
                    .ok()
                    .and_then(|ov| u32::try_from(zbus::zvariant::Value::from(ov)).ok())
                    .unwrap_or(0);
                
                let security = if rsn_flags & 0x100 != 0 {
                    SecurityType::WPA3
                } else if rsn_flags != 0 {
                    SecurityType::WPA2
                } else if wpa_flags != 0 {
                    SecurityType::WPA
                } else if flags != 0 {
                    SecurityType::WEP
                } else {
                    SecurityType::None
                };
                
                let is_connected = active_ssid.as_ref() == Some(&ssid);
                
                access_points.push(AccessPoint {
                    ssid,
                    signal_strength: strength,
                    security,
                    is_connected,
                    device_path: device_path.clone(),
                    path: ap_path.to_string(),
                });
            }
        }
        
        access_points.sort_by(|a, b| b.signal_strength.cmp(&a.signal_strength));
        
        let mut seen_ssids: std::collections::HashSet<String> = std::collections::HashSet::new();
        let mut unique_aps: Vec<AccessPoint> = Vec::new();
        
        for ap in access_points {
            if !seen_ssids.contains(&ap.ssid) {
                seen_ssids.insert(ap.ssid.clone());
                unique_aps.push(ap);
            } else if ap.is_connected {
                let existing = unique_aps.iter_mut().find(|x| x.ssid == ap.ssid);
                if let Some(existing) = existing {
                    existing.is_connected = true;
                }
            }
        }
        
        Ok(unique_aps)
    }
    
    async fn get_ap_property(&self, ap_path: &str, property: &str) -> zbus::Result<zbus::zvariant::OwnedValue> {
        let path: zbus::zvariant::ObjectPath = ap_path.try_into()
            .map_err(|e: zbus::zvariant::Error| zbus::Error::Variant(e))?;
        let reply = self.conn
            .call_method(
                Some("org.freedesktop.NetworkManager"),
                &path,
                Some("org.freedesktop.DBus.Properties"),
                "Get",
                &("org.freedesktop.NetworkManager.AccessPoint", property),
            )
            .await?
            .body()
            .deserialize::<zbus::zvariant::OwnedValue>()?;
        
        Ok(reply)
    }
    
    pub async fn get_active_ssid(&self) -> Option<String> {
        let active_paths = self.get_active_connection_paths().await;
        
        for path in active_paths {
            let path_obj = match zbus::zvariant::ObjectPath::try_from(path.as_str()) {
                Ok(p) => p,
                Err(_) => continue,
            };
            
            let id_reply = self.conn
                .call_method(
                    Some("org.freedesktop.NetworkManager"),
                    &path_obj,
                    Some("org.freedesktop.DBus.Properties"),
                    "Get",
                    &("org.freedesktop.NetworkManager.Connection.Active", "Id"),
                )
                .await;

            if let Ok(reply) = id_reply {
                if let Ok(id_val) = reply.body().deserialize::<zbus::zvariant::OwnedValue>() {
                    let val: zbus::zvariant::Value = id_val.into();
                    if let Ok(id) = String::try_from(val) {
                        return Some(id);
                    }
                }
            }
        }
        
        None
    }

    pub async fn has_saved_connection(&self, ssid: &str) -> bool {
        self.find_connection_by_ssid(ssid).await.is_some()
    }
    
    async fn find_connection_by_ssid(&self, ssid: &str) -> Option<String> {
        let connections_reply = self.conn
            .call_method(
                Some("org.freedesktop.NetworkManager"),
                "/org/freedesktop/NetworkManager/Settings",
                Some("org.freedesktop.NetworkManager.Settings"),
                "ListConnections",
                &(),
            )
            .await;

        let connections: Vec<zbus::zvariant::OwnedObjectPath> = match connections_reply {
            Ok(r) => r.body().deserialize().unwrap_or_default(),
            Err(e) => {
                log::error!("Failed to list connections: {}", e);
                return None;
            },
        };
        
        for conn_path in connections {
            if let Ok(settings) = self.get_connection_settings_raw(&conn_path).await {
                // Check for 802-11-wireless.ssid
                if let Some(wireless_map) = settings.get("802-11-wireless") {
                    if let Some(v) = wireless_map.get("ssid") {
                        let ssid_bytes = if let zbus::zvariant::Value::Array(a) = &**v {
                            a.iter().filter_map(|iv| u8::try_from(iv).ok()).collect::<Vec<u8>>()
                        } else {
                            Vec::new()
                        };
                        
                        if !ssid_bytes.is_empty() {
                            let stored_ssid = String::from_utf8_lossy(&ssid_bytes).to_string();
                            // Direct byte comparison or trimmed string comparison
                            if stored_ssid == ssid || stored_ssid.trim() == ssid.trim() || ssid_bytes == ssid.as_bytes() {
                                return Some(conn_path.to_string());
                            }
                        }
                    }
                }
                
                // Also check connection.id (SSID name often used here)
                if let Some(connection_map) = settings.get("connection") {
                    if let Some(id_owned) = connection_map.get("id") {
                        let val: &zbus::zvariant::Value = &**id_owned;
                        if let Ok(id) = <&str>::try_from(val) {
                            if id == ssid || id.trim() == ssid.trim() {
                                return Some(conn_path.to_string());
                            }
                        }
                    }
                }
            }
        }
        None
    }

    pub async fn connect_to_network(&self, ssid: &str, password: Option<&str>, device_path: &str) -> zbus::Result<()> {
        let dev_path: zbus::zvariant::ObjectPath = device_path.try_into()
            .map_err(|e: zbus::zvariant::Error| zbus::Error::Variant(e))?;

        if let Some(existing_path_str) = self.find_connection_by_ssid(ssid).await {
            let existing_path = zbus::zvariant::ObjectPath::try_from(existing_path_str.as_str()).unwrap();
            let specific_object = zbus::zvariant::ObjectPath::try_from("/").unwrap();
            
            self.conn.call_method(
                Some("org.freedesktop.NetworkManager"),
                "/org/freedesktop/NetworkManager",
                Some("org.freedesktop.NetworkManager"),
                "ActivateConnection",
                &(&existing_path, &dev_path, &specific_object),
            ).await?;
        } else {
            let mut connection: HashMap<&str, zbus::zvariant::Value> = HashMap::new();
            connection.insert("type", "802-11-wireless".into());
            connection.insert("id", ssid.into());
            connection.insert("uuid", zbus::zvariant::Value::Str(uuid::Uuid::new_v4().to_string().into()));
            connection.insert("autoconnect", true.into());
            
            let mut wireless: HashMap<&str, zbus::zvariant::Value> = HashMap::new();
            wireless.insert("ssid", ssid.as_bytes().into());
            wireless.insert("mode", "infrastructure".into());
            
            let mut config: HashMap<&str, HashMap<&str, zbus::zvariant::Value>> = HashMap::new();
            config.insert("connection", connection);
            config.insert("802-11-wireless", wireless);
            
            if let Some(pwd) = password {
                let mut wsec: HashMap<&str, zbus::zvariant::Value> = HashMap::new();
                wsec.insert("key-mgmt", "wpa-psk".into());
                wsec.insert("auth-alg", "open".into());
                wsec.insert("psk", pwd.into());
                config.insert("802-11-wireless-security", wsec);
            }
            
            let mut ipv4: HashMap<&str, zbus::zvariant::Value> = HashMap::new();
            ipv4.insert("method", "auto".into());
            config.insert("ipv4", ipv4);
            
            let mut ipv6: HashMap<&str, zbus::zvariant::Value> = HashMap::new();
            ipv6.insert("method", "ignore".into());
            config.insert("ipv6", ipv6);
            
            let specific_object = zbus::zvariant::ObjectPath::try_from("/").unwrap();
            self.conn
                .call_method(
                    Some("org.freedesktop.NetworkManager"),
                    "/org/freedesktop/NetworkManager",
                    Some("org.freedesktop.NetworkManager"),
                    "AddAndActivateConnection",
                    &(&config, &dev_path, &specific_object),
                )
                .await?;
        }
        
        let mut retries = 0;
        while retries < 30 {
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
            if let Some(current) = self.get_active_ssid().await {
                if current == ssid {
                    return Ok(());
                }
            }
            retries += 1;
        }
        Err(zbus::Error::Address("Connection timeout".to_string()))
    }

    pub async fn connect_hidden(&self, ssid: &str, password: Option<&str>, device_path: &str) -> zbus::Result<()> {
        let mut connection: HashMap<&str, zbus::zvariant::Value> = HashMap::new();
        connection.insert("type", "802-11-wireless".into());
        connection.insert("id", ssid.into());
        connection.insert("uuid", zbus::zvariant::Value::Str(uuid::Uuid::new_v4().to_string().into()));
        connection.insert("autoconnect", true.into());
        
        let mut wireless: HashMap<&str, zbus::zvariant::Value> = HashMap::new();
        wireless.insert("ssid", ssid.as_bytes().into());
        wireless.insert("mode", "infrastructure".into());
        wireless.insert("hidden", true.into());
        
        let mut config: HashMap<&str, HashMap<&str, zbus::zvariant::Value>> = HashMap::new();
        config.insert("connection", connection);
        config.insert("802-11-wireless", wireless);
        
        if let Some(pwd) = password {
            let mut wsec: HashMap<&str, zbus::zvariant::Value> = HashMap::new();
            wsec.insert("key-mgmt", "wpa-psk".into());
            wsec.insert("auth-alg", "open".into());
            wsec.insert("psk", pwd.into());
            config.insert("802-11-wireless-security", wsec);
        }
        
        let mut ipv4: HashMap<&str, zbus::zvariant::Value> = HashMap::new();
        ipv4.insert("method", "auto".into());
        config.insert("ipv4", ipv4);
        
        let mut ipv6: HashMap<&str, zbus::zvariant::Value> = HashMap::new();
        ipv6.insert("method", "ignore".into());
        config.insert("ipv6", ipv6);
        
        let dev_path: zbus::zvariant::ObjectPath = device_path.try_into()
            .map_err(|e: zbus::zvariant::Error| zbus::Error::Variant(e))?;
        
        let specific_object = zbus::zvariant::ObjectPath::try_from("/").unwrap();

        self.conn
            .call_method(
                Some("org.freedesktop.NetworkManager"),
                "/org/freedesktop/NetworkManager",
                Some("org.freedesktop.NetworkManager"),
                "AddAndActivateConnection",
                &(&config, &dev_path, &specific_object),
            )
            .await?;
        Ok(())
    }
    
    pub async fn disconnect_ap(&self, ssid: &str, _ap_path: &str) -> zbus::Result<()> {
        let active_paths = self.get_active_connection_paths().await;
        for path_str in active_paths {
            let path = match zbus::zvariant::ObjectPath::try_from(path_str.as_str()) {
                Ok(p) => p,
                Err(_) => continue,
            };

            let id_reply = self.conn
                .call_method(
                    Some("org.freedesktop.NetworkManager"),
                    &path,
                    Some("org.freedesktop.DBus.Properties"),
                    "Get",
                    &("org.freedesktop.NetworkManager.Connection.Active", "Id"),
                )
                .await;

            let id_val = match id_reply {
                Ok(r) => r.body().deserialize::<zbus::zvariant::OwnedValue>().ok(),
                Err(_) => None,
            };
            
            let id = id_val
                .and_then(|v| String::try_from(zbus::zvariant::Value::from(v)).ok())
                .unwrap_or_default();

            if id == ssid {
                self.conn
                    .call_method(
                        Some("org.freedesktop.NetworkManager"),
                        "/org/freedesktop/NetworkManager",
                        Some("org.freedesktop.NetworkManager"),
                        "DeactivateConnection",
                        &(path),
                    )
                    .await?;
                return Ok(());
            }
        }
        Ok(())
    }

    pub async fn forget_network(&self, path: &str) -> zbus::Result<()> {
        let path_obj: zbus::zvariant::ObjectPath = path.try_into()
            .map_err(|e: zbus::zvariant::Error| zbus::Error::Variant(e))?;
        self.conn
            .call_method(
                Some("org.freedesktop.NetworkManager"),
                &path_obj,
                Some("org.freedesktop.NetworkManager.Settings.Connection"),
                "Delete",
                &(),
            )
            .await?;
        Ok(())
    }

    pub async fn get_saved_networks(&self) -> zbus::Result<Vec<SavedNetwork>> {
        let connections_reply = self.conn
            .call_method(
                Some("org.freedesktop.NetworkManager"),
                "/org/freedesktop/NetworkManager/Settings",
                Some("org.freedesktop.NetworkManager.Settings"),
                "ListConnections",
                &(),
            )
            .await?;
        
        let connections: Vec<zbus::zvariant::OwnedObjectPath> = connections_reply.body().deserialize()?;
        let mut saved_networks = Vec::new();
        let active_connections = self.get_active_connection_paths().await;
        
        for conn_path in connections {
            if let Ok(settings) = self.get_connection_settings_raw(&conn_path).await {
                if let Some(connection_map) = settings.get("connection") {
                    let id = connection_map.get("id")
                        .and_then(|v| <&str>::try_from(&**v).ok())
                        .unwrap_or_default()
                        .to_string();
                    
                    let conn_type = connection_map.get("type")
                        .and_then(|v| <&str>::try_from(&**v).ok())
                        .unwrap_or_default();
                    
                    if conn_type == "802-11-wireless" {
                        let autoconnect = connection_map.get("autoconnect")
                            .and_then(|v| bool::try_from(&**v).ok())
                            .unwrap_or(true);
                        
                        let is_active = active_connections.contains(&conn_path.to_string());
                        
                        saved_networks.push(SavedNetwork {
                            ssid: id,
                            path: conn_path.to_string(),
                            autoconnect,
                            is_active,
                        });
                    }
                }
            }
        }
        saved_networks.sort_by(|a, b| b.is_active.cmp(&a.is_active).then_with(|| a.ssid.cmp(&b.ssid)));
        Ok(saved_networks)
    }

    async fn get_connection_settings_raw(&self, path: &zbus::zvariant::OwnedObjectPath) -> zbus::Result<HashMap<String, HashMap<String, zbus::zvariant::OwnedValue>>> {
        self.conn
            .call_method(
                Some("org.freedesktop.NetworkManager"),
                path,
                Some("org.freedesktop.NetworkManager.Settings.Connection"),
                "GetSettings",
                &(),
            )
            .await?
            .body()
            .deserialize()
    }

    async fn get_connection_settings_from_path(&self, path: &zbus::zvariant::ObjectPath<'_>) -> zbus::Result<HashMap<String, HashMap<String, zbus::zvariant::OwnedValue>>> {
        self.conn
            .call_method(
                Some("org.freedesktop.NetworkManager"),
                path,
                Some("org.freedesktop.NetworkManager.Settings.Connection"),
                "GetSettings",
                &(),
            )
            .await?
            .body()
            .deserialize()
    }
    
    async fn get_active_connection_paths(&self) -> Vec<String> {
        let reply = self.conn
            .call_method(
                Some("org.freedesktop.NetworkManager"),
                "/org/freedesktop/NetworkManager",
                Some("org.freedesktop.DBus.Properties"),
                "Get",
                &("org.freedesktop.NetworkManager", "ActiveConnections"),
            )
            .await;
        if let Ok(r) = reply {
            if let Ok(val) = r.body().deserialize::<zbus::zvariant::OwnedValue>() {
                if let Ok(paths) = <Vec<zbus::zvariant::OwnedObjectPath>>::try_from(val) {
                    return paths.into_iter().map(|p| p.to_string()).collect();
                }
            }
        }
        Vec::new()
    }
    
    pub async fn set_autoconnect(&self, path: &str, autoconnect: bool) -> zbus::Result<()> {
        let path_obj: zbus::zvariant::ObjectPath = path.try_into()
            .map_err(|e: zbus::zvariant::Error| zbus::Error::Variant(e))?;
        let current_settings = self.get_connection_settings_from_path(&path_obj).await?;
        let mut new_settings: HashMap<String, HashMap<String, zbus::zvariant::Value>> = HashMap::new();
        for (group_name, group_settings) in current_settings {
            let mut new_group: HashMap<String, zbus::zvariant::Value> = HashMap::new();
            for (key, value) in group_settings {
                new_group.insert(key, zbus::zvariant::Value::from(value));
            }
            new_settings.insert(group_name, new_group);
        }
        if let Some(conn_group) = new_settings.get_mut("connection") {
            conn_group.insert("autoconnect".to_string(), zbus::zvariant::Value::Bool(autoconnect));
        }
        self.conn
            .call_method(
                Some("org.freedesktop.NetworkManager"),
                &path_obj,
                Some("org.freedesktop.NetworkManager.Settings.Connection"),
                "Update",
                &(&new_settings),
            )
            .await?;
        Ok(())
    }
    
    pub async fn get_network_details(&self, ssid: &str) -> zbus::Result<NetworkDetails> {
        let mut details = NetworkDetails {
            ssid: ssid.to_string(),
            ..Default::default()
        };
        let active_paths = self.get_active_connection_paths().await;
        for path_str in active_paths {
            let path = match zbus::zvariant::ObjectPath::try_from(path_str.as_str()) {
                Ok(p) => p,
                Err(_) => continue,
            };
            let id_val_reply: zbus::zvariant::OwnedValue = self.conn
                .call_method(
                    Some("org.freedesktop.NetworkManager"),
                    &path,
                    Some("org.freedesktop.DBus.Properties"),
                    "Get",
                    &("org.freedesktop.NetworkManager.Connection.Active", "Id"),
                )
                .await?
                .body()
                .deserialize()?;
            
            let id = String::try_from(zbus::zvariant::Value::from(id_val_reply)).unwrap_or_default();
            
            if id == ssid {
                details.is_connected = true;
                let ip4_val_reply: zbus::zvariant::OwnedValue = self.conn
                    .call_method(
                        Some("org.freedesktop.NetworkManager"),
                        &path,
                        Some("org.freedesktop.DBus.Properties"),
                        "Get",
                        &("org.freedesktop.NetworkManager.Connection.Active", "Ip4Config"),
                    )
                    .await?
                    .body()
                    .deserialize()?;
                
                let ip4_path = zbus::zvariant::OwnedObjectPath::try_from(ip4_val_reply).unwrap_or_else(|_| "/".try_into().unwrap());
                
                if ip4_path.as_str() != "/" {
                    let addr_reply_val: zbus::zvariant::OwnedValue = self.conn
                        .call_method(
                            Some("org.freedesktop.NetworkManager"),
                            &ip4_path,
                            Some("org.freedesktop.DBus.Properties"),
                            "Get",
                            &("org.freedesktop.NetworkManager.IP4Config", "AddressData"),
                        )
                        .await?
                        .body()
                        .deserialize()?;
                    
                    let val: zbus::zvariant::Value = addr_reply_val.into();
                    if let zbus::zvariant::Value::Array(a) = val {
                        for iv in a.iter() {
                            let owned_iv = zbus::zvariant::OwnedValue::try_from(iv).expect("Value should be convertible to OwnedValue");
                            if let Ok(map) = HashMap::<String, zbus::zvariant::OwnedValue>::try_from(owned_iv) {
                                if let Some(address_v) = map.get("address") {
                                    if let Ok(addr_str) = <&str>::try_from(&**address_v) {
                                        details.ip4_address = addr_str.to_string();
                                    }
                                }
                            }
                        }
                    }
                    
                    let gateway_val_reply: zbus::zvariant::OwnedValue = self.conn
                        .call_method(
                            Some("org.freedesktop.NetworkManager"),
                            &ip4_path,
                            Some("org.freedesktop.DBus.Properties"),
                            "Get",
                            &("org.freedesktop.NetworkManager.IP4Config", "Gateway"),
                        )
                        .await?
                        .body()
                        .deserialize()?;
                    details.gateway = String::try_from(zbus::zvariant::Value::from(gateway_val_reply)).unwrap_or_default();
                    
                    let dns_reply_val: zbus::zvariant::OwnedValue = self.conn
                        .call_method(
                            Some("org.freedesktop.NetworkManager"),
                            &ip4_path,
                            Some("org.freedesktop.DBus.Properties"),
                            "Get",
                            &("org.freedesktop.NetworkManager.IP4Config", "Nameservers"),
                        )
                        .await?
                        .body()
                        .deserialize()?;
                    
                    let dns_val: zbus::zvariant::Value = dns_reply_val.into();
                    if let zbus::zvariant::Value::Array(a) = dns_val {
                        for iv in a.iter() {
                            if let zbus::zvariant::Value::Array(ba) = iv {
                                let bytes: Vec<u8> = ba.iter().filter_map(|bv| {
                                    u8::try_from(bv).ok()
                                }).collect();
                                if bytes.len() == 4 {
                                    details.dns_servers.push(format!("{}.{}.{}.{}", bytes[0], bytes[1], bytes[2], bytes[3]));
                                }
                            }
                        }
                    }
                }
                
                let dev_reply_val: zbus::zvariant::OwnedValue = self.conn
                    .call_method(
                        Some("org.freedesktop.NetworkManager"),
                        &path,
                        Some("org.freedesktop.DBus.Properties"),
                        "Get",
                        &("org.freedesktop.NetworkManager.Connection.Active", "Devices"),
                    )
                    .await?
                    .body()
                    .deserialize()?;
                
                let dev_val: zbus::zvariant::Value = dev_reply_val.into();
                if let zbus::zvariant::Value::Array(a) = dev_val {
                    for iv in a.iter() {
                        let owned_iv = zbus::zvariant::OwnedValue::try_from(iv).expect("Value should be convertible to OwnedValue");
                        if let Ok(device_path) = zbus::zvariant::OwnedObjectPath::try_from(owned_iv) {
                             let hw_val_reply: zbus::zvariant::OwnedValue = self.conn
                                .call_method(
                                    Some("org.freedesktop.NetworkManager"),
                                    &device_path,
                                    Some("org.freedesktop.DBus.Properties"),
                                    "Get",
                                    &("org.freedesktop.NetworkManager.Device", "HwAddress"),
                                )
                                .await?
                                .body()
                                .deserialize()?;
                            details.mac_address = String::try_from(zbus::zvariant::Value::from(hw_val_reply)).unwrap_or_default();
                            break;
                        }
                    }
                }
                break;
            }
        }
        Ok(details)
    }
    pub async fn get_saved_vpns(&self) -> zbus::Result<Vec<VpnConnection>> {
        let connections_reply = self.conn
            .call_method(
                Some("org.freedesktop.NetworkManager"),
                "/org/freedesktop/NetworkManager/Settings",
                Some("org.freedesktop.NetworkManager.Settings"),
                "ListConnections",
                &(),
            )
            .await?;

        let connections: Vec<zbus::zvariant::OwnedObjectPath> = connections_reply.body().deserialize()?;
        let mut vpn_list = Vec::new();
        let active_paths = self.get_active_connection_paths().await;

        for conn_path in connections {
            println!("Checking connection: {}", conn_path);
            if let Ok(settings) = self.get_connection_settings_raw(&conn_path).await {
                if let Some(connection_map) = settings.get("connection") {
                    let conn_type = connection_map.get("type")
                        .and_then(|v| v.downcast_ref::<String>().ok())
                        .unwrap_or_default();
                    println!("Connection type: {}", conn_type);

                    // Szűrünk a vpn és wireguard típusokra
                    if conn_type == "vpn" || conn_type == "wireguard" {
                        println!("Found VPN connection: {}", conn_path);
                        let id = connection_map.get("id")
                            .and_then(|v| v.downcast_ref::<String>().ok())
                            .unwrap_or_default();
                        
                        let uuid = connection_map.get("uuid")
                            .and_then(|v| v.downcast_ref::<String>().ok())
                            .unwrap_or_default();


                        let is_active = active_paths.contains(&conn_path.to_string());
                        println!("Is active: {}", is_active);

                        vpn_list.push(VpnConnection {
                            name: id,
                            uuid,
                            path: conn_path.to_string(),
                            connection_type: conn_type,
                            is_active,
                        });
                    }
                }
            }
        }
        Ok(vpn_list)
    }
}
