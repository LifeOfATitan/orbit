use zbus::Connection;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct AccessPoint {
    pub ssid: String,
    pub signal_strength: u8,
    pub security: SecurityType,
    pub is_connected: bool,
    pub device_path: String,
}

#[derive(Debug, Clone)]
pub struct SavedNetwork {
    pub ssid: String,
    pub path: String,
    pub autoconnect: bool,
    pub is_active: bool,
}

#[derive(Debug, Clone, Default)]
pub struct NetworkDetails {
    pub ssid: String,
    pub ip4_address: String,
    pub gateway: String,
    pub dns_servers: Vec<String>,
    pub mac_address: String,
    pub connection_speed: String,
    pub is_connected: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SecurityType {
    None,
    WEP,
    WPA,
    WPA2,
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
    
    async fn get_wireless_devices(&self) -> zbus::Result<Vec<String>> {
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
            
            let device_type: u32 = u32::try_from(dtype_reply).unwrap_or(0);
            
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
                
                let ssid_bytes: Vec<u8> = self.get_ap_property(ap_path.as_str(), "Ssid").await
                    .ok()
                    .and_then(|v| Vec::<u8>::try_from(v).ok())
                    .unwrap_or_default();
                
                let ssid = String::from_utf8_lossy(&ssid_bytes).to_string();
                
                if ssid.is_empty() {
                    continue;
                }
                
                let strength: u8 = self.get_ap_property(ap_path.as_str(), "Strength").await
                    .ok()
                    .and_then(|v| u8::try_from(v).ok())
                    .unwrap_or(0);
                let flags: u32 = self.get_ap_property(ap_path.as_str(), "Flags").await
                    .ok()
                    .and_then(|v| u32::try_from(v).ok())
                    .unwrap_or(0);
                let rsn_flags: u32 = self.get_ap_property(ap_path.as_str(), "RsnFlags").await
                    .ok()
                    .and_then(|v| u32::try_from(v).ok())
                    .unwrap_or(0);
                let wpa_flags: u32 = self.get_ap_property(ap_path.as_str(), "WpaFlags").await
                    .ok()
                    .and_then(|v| u32::try_from(v).ok())
                    .unwrap_or(0);
                
                let security = if rsn_flags != 0 {
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
        let reply = self.conn
            .call_method(
                Some("org.freedesktop.NetworkManager"),
                "/org/freedesktop/NetworkManager",
                Some("org.freedesktop.DBus.Properties"),
                "Get",
                &("org.freedesktop.NetworkManager", "ActiveConnections"),
            )
            .await
            .ok()?
            .body()
            .deserialize::<zbus::zvariant::OwnedValue>()
            .ok()?;
        
        let active_paths: Vec<zbus::zvariant::OwnedObjectPath> = reply.try_into().ok()?;
        
        for path in active_paths {
            if let Ok(id_reply) = self.conn
                .call_method(
                    Some("org.freedesktop.NetworkManager"),
                    &path,
                    Some("org.freedesktop.DBus.Properties"),
                    "Get",
                    &("org.freedesktop.NetworkManager.Connection.Active", "Id"),
                )
                .await
            {
                if let Ok(id_val) = id_reply.body().deserialize::<zbus::zvariant::OwnedValue>() {
                    if let Ok(id) = String::try_from(id_val) {
                        return Some(id);
                    }
                }
            }
        }
        
        None
    }
    
    pub async fn connect(&self, ssid: &str, password: Option<&str>, device_path: &str) -> zbus::Result<()> {
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
        
        let dev_path: zbus::zvariant::ObjectPath = device_path.try_into()
            .map_err(|e: zbus::zvariant::Error| zbus::Error::Variant(e))?;
        
        self.conn
            .call_method(
                Some("org.freedesktop.NetworkManager"),
                "/org/freedesktop/NetworkManager",
                Some("org.freedesktop.NetworkManager"),
                "AddAndActivateConnection",
                &(&config, &dev_path, "/"),
            )
            .await?;
        
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
    
    pub async fn disconnect(&self) -> zbus::Result<()> {
        let reply = self.conn
            .call_method(
                Some("org.freedesktop.NetworkManager"),
                "/org/freedesktop/NetworkManager",
                Some("org.freedesktop.DBus.Properties"),
                "Get",
                &("org.freedesktop.NetworkManager", "ActiveConnections"),
            )
            .await?
            .body()
            .deserialize::<zbus::zvariant::OwnedValue>()?;
        
        let active_paths: Vec<zbus::zvariant::OwnedObjectPath> = reply.try_into().unwrap_or_default();
        
        for path in active_paths {
            self.conn
                .call_method(
                    Some("org.freedesktop.NetworkManager"),
                    "/org/freedesktop/NetworkManager",
                    Some("org.freedesktop.NetworkManager"),
                    "DeactivateConnection",
                    &path.as_str(),
                )
                .await?;
        }
        
        Ok(())
    }
    
    pub async fn get_saved_networks(&self) -> zbus::Result<Vec<SavedNetwork>> {
        let connections: Vec<zbus::zvariant::OwnedObjectPath> = self.conn
            .call_method(
                Some("org.freedesktop.NetworkManager"),
                "/org/freedesktop/NetworkManager/Settings",
                Some("org.freedesktop.NetworkManager.Settings"),
                "ListConnections",
                &(),
            )
            .await?
            .body()
            .deserialize()?;
        
        let mut saved_networks = Vec::new();
        let active_connections = self.get_active_connection_paths().await;
        
        for conn_path in connections {
            if let Ok(settings) = self.get_connection_settings(&conn_path).await {
                if let Some(connection_map) = settings.get("connection") {
                    let conn_type = connection_map.get("type")
                        .and_then(|v| v.downcast_ref::<String>().ok())
                        .map(|s| s.to_string())
                        .unwrap_or_default();
                    
                    if conn_type == "802-11-wireless" {
                        let id = connection_map.get("id")
                            .and_then(|v| v.downcast_ref::<String>().ok())
                            .map(|s| s.to_string())
                            .unwrap_or_default();
                        
                        let autoconnect = connection_map.get("autoconnect")
                            .and_then(|v| v.downcast_ref::<bool>().ok())
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
        
        saved_networks.sort_by(|a, b| {
            b.is_active.cmp(&a.is_active)
                .then_with(|| a.ssid.cmp(&b.ssid))
        });
        
        Ok(saved_networks)
    }
    
    async fn get_connection_settings(&self, path: &zbus::zvariant::OwnedObjectPath) -> zbus::Result<HashMap<String, HashMap<String, zbus::zvariant::OwnedValue>>> {
        let path_obj: zbus::zvariant::ObjectPath = path.as_str().try_into()
            .map_err(|e: zbus::zvariant::Error| zbus::Error::Variant(e))?;
        
        let settings: HashMap<String, HashMap<String, zbus::zvariant::OwnedValue>> = self.conn
            .call_method(
                Some("org.freedesktop.NetworkManager"),
                &path_obj,
                Some("org.freedesktop.NetworkManager.Settings.Connection"),
                "GetSettings",
                &(),
            )
            .await?
            .body()
            .deserialize()?;
        
        Ok(settings)
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
            .await
            .ok()
            .and_then(|r| r.body().deserialize::<zbus::zvariant::OwnedValue>().ok());
        
        if let Some(reply) = reply {
            if let Ok(paths) = <Vec<zbus::zvariant::OwnedObjectPath>>::try_from(reply) {
                return paths.iter().map(|p| p.to_string()).collect();
            }
        }
        
        Vec::new()
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
    
    pub async fn set_autoconnect(&self, path: &str, autoconnect: bool) -> zbus::Result<()> {
        let path_obj: zbus::zvariant::ObjectPath = path.try_into()
            .map_err(|e: zbus::zvariant::Error| zbus::Error::Variant(e))?;
        
        // First get current settings
        let current_settings: HashMap<String, HashMap<String, zbus::zvariant::OwnedValue>> = self.conn
            .call_method(
                Some("org.freedesktop.NetworkManager"),
                &path_obj,
                Some("org.freedesktop.NetworkManager.Settings.Connection"),
                "GetSettings",
                &(),
            )
            .await?
            .body()
            .deserialize()?;
        
        // Convert to the format needed for Update
        let mut new_settings: HashMap<String, HashMap<String, zbus::zvariant::Value>> = HashMap::new();
        
        for (group_name, group_settings) in current_settings {
            let mut new_group: HashMap<String, zbus::zvariant::Value> = HashMap::new();
            for (key, value) in group_settings {
                // Convert OwnedValue to Value
                new_group.insert(key, zbus::zvariant::Value::from(value));
            }
            new_settings.insert(group_name, new_group);
        }
        
        // Modify the autoconnect field
        if let Some(conn_group) = new_settings.get_mut("connection") {
            conn_group.insert(
                "autoconnect".to_string(),
                zbus::zvariant::Value::Bool(autoconnect)
            );
        }
        
        // Update with all settings
        self.conn
            .call_method(
                Some("org.freedesktop.NetworkManager"),
                &path_obj,
                Some("org.freedesktop.NetworkManager.Settings.Connection"),
                "Update",
                &(&new_settings as &HashMap<String, HashMap<String, zbus::zvariant::Value>>),
            )
            .await?;
        
        Ok(())
    }
    
    pub async fn get_network_details(&self, ssid: &str) -> zbus::Result<NetworkDetails> {
        let mut details = NetworkDetails {
            ssid: ssid.to_string(),
            ..Default::default()
        };
        
        // Find active connection by SSID
        let active_paths = self.get_active_connection_paths().await;
        
        for path_str in active_paths {
            let path: zbus::zvariant::ObjectPath = path_str.as_str().try_into()
                .map_err(|e: zbus::zvariant::Error| zbus::Error::Variant(e))?;
            
            // Get connection ID (SSID)
            let id: String = self.conn
                .call_method(
                    Some("org.freedesktop.NetworkManager"),
                    &path,
                    Some("org.freedesktop.DBus.Properties"),
                    "Get",
                    &("org.freedesktop.NetworkManager.Connection.Active", "Id"),
                )
                .await?
                .body()
                .deserialize::<zbus::zvariant::OwnedValue>()?
                .try_into()
                .unwrap_or_default();
            
            if id == ssid {
                details.is_connected = true;
                
                // Get IP4Config path
                let ip4_path: zbus::zvariant::OwnedObjectPath = self.conn
                    .call_method(
                        Some("org.freedesktop.NetworkManager"),
                        &path,
                        Some("org.freedesktop.DBus.Properties"),
                        "Get",
                        &("org.freedesktop.NetworkManager.Connection.Active", "Ip4Config"),
                    )
                    .await?
                    .body()
                    .deserialize::<zbus::zvariant::OwnedValue>()?
                    .try_into()
                    .unwrap_or_else(|_| "/".try_into().unwrap());
                
                if ip4_path.as_str() != "/" {
                    // Get IP address from AddressData
                    let addr_data: Vec<HashMap<String, zbus::zvariant::OwnedValue>> = self.conn
                        .call_method(
                            Some("org.freedesktop.NetworkManager"),
                            &ip4_path,
                            Some("org.freedesktop.DBus.Properties"),
                            "Get",
                            &("org.freedesktop.NetworkManager.IP4Config", "AddressData"),
                        )
                        .await?
                        .body()
                        .deserialize::<zbus::zvariant::OwnedValue>()?
                        .try_into()
                        .unwrap_or_default();
                    
                    if let Some(first_addr) = addr_data.first() {
                        if let Some(address) = first_addr.get("address") {
                            if let Ok(addr_str) = <&str>::try_from(address) {
                                details.ip4_address = addr_str.to_string();
                            }
                        }
                    }
                    
                    // Get gateway
                    let gateway: String = self.conn
                        .call_method(
                            Some("org.freedesktop.NetworkManager"),
                            &ip4_path,
                            Some("org.freedesktop.DBus.Properties"),
                            "Get",
                            &("org.freedesktop.NetworkManager.IP4Config", "Gateway"),
                        )
                        .await?
                        .body()
                        .deserialize::<zbus::zvariant::OwnedValue>()?
                        .try_into()
                        .unwrap_or_default();
                    details.gateway = gateway;
                    
                    // Get DNS servers - array of byte arrays (IP addresses in network byte order)
                    let dns_data: Vec<Vec<u8>> = self.conn
                        .call_method(
                            Some("org.freedesktop.NetworkManager"),
                            &ip4_path,
                            Some("org.freedesktop.DBus.Properties"),
                            "Get",
                            &("org.freedesktop.NetworkManager.IP4Config", "Nameservers"),
                        )
                        .await?
                        .body()
                        .deserialize::<zbus::zvariant::OwnedValue>()?
                        .try_into()
                        .unwrap_or_default();
                    
                    for dns_bytes in dns_data {
                        if dns_bytes.len() == 4 {
                            details.dns_servers.push(format!(
                                "{}.{}.{}.{}",
                                dns_bytes[0], dns_bytes[1], dns_bytes[2], dns_bytes[3]
                            ));
                        }
                    }
                }
                
                // Get device for MAC address
                let devices: Vec<zbus::zvariant::OwnedObjectPath> = self.conn
                    .call_method(
                        Some("org.freedesktop.NetworkManager"),
                        &path,
                        Some("org.freedesktop.DBus.Properties"),
                        "Get",
                        &("org.freedesktop.NetworkManager.Connection.Active", "Devices"),
                    )
                    .await?
                    .body()
                    .deserialize::<zbus::zvariant::OwnedValue>()?
                    .try_into()
                    .unwrap_or_default();
                
                if let Some(device_path) = devices.first() {
                    let hw_addr: String = self.conn
                        .call_method(
                            Some("org.freedesktop.NetworkManager"),
                            device_path,
                            Some("org.freedesktop.DBus.Properties"),
                            "Get",
                            &("org.freedesktop.NetworkManager.Device", "HwAddress"),
                        )
                        .await?
                        .body()
                        .deserialize::<zbus::zvariant::OwnedValue>()?
                        .try_into()
                        .unwrap_or_default();
                    details.mac_address = hw_addr;
                    
                    // Get bit rate (connection speed) for wireless devices
                    let bitrate_result = self.conn
                        .call_method(
                            Some("org.freedesktop.NetworkManager"),
                            device_path,
                            Some("org.freedesktop.DBus.Properties"),
                            "Get",
                            &("org.freedesktop.NetworkManager.Device.Wireless", "Bitrate"),
                        )
                        .await;
                    
                    if let Ok(reply) = bitrate_result {
                        if let Ok(bitrate_val) = reply.body().deserialize::<zbus::zvariant::OwnedValue>() {
                            if let Ok(bitrate) = u32::try_from(bitrate_val) {
                                details.connection_speed = format!("{} Mbps", bitrate / 1000);
                            }
                        }
                    }
                }
                
                break;
            }
        }
        
        Ok(details)
    }
}
