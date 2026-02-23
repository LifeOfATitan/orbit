use zbus::Connection;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum DeviceType {
    Unknown,
    Phone,
    Computer,
    Audio,
    Headphones,
    Keyboard,
    Mouse,
    Other,
}

impl DeviceType {
    pub fn icon_name(&self) -> &'static str {
        match self {
            DeviceType::Phone => "smartphone-symbolic",
            DeviceType::Computer => "computer-symbolic",
            DeviceType::Audio => "audio-speakers-symbolic",
            DeviceType::Headphones => "audio-headphones-symbolic",
            DeviceType::Keyboard => "input-keyboard-symbolic",
            DeviceType::Mouse => "input-mouse-symbolic",
            _ => "bluetooth-symbolic",
        }
    }
}

impl From<u32> for DeviceType {
    fn from(value: u32) -> Self {
        match value {
            1 | 2 => DeviceType::Computer,
            3 => DeviceType::Phone,
            4 | 5 | 6 => DeviceType::Audio,
            7 => DeviceType::Headphones,
            8 => DeviceType::Keyboard,
            9 => DeviceType::Mouse,
            _ => DeviceType::Other,
        }
    }
}

#[derive(Debug, Clone)]
pub struct BluetoothDevice {
    pub path: String,
    pub name: String,
    pub address: String,
    pub device_type: DeviceType,
    pub is_connected: bool,
    pub is_paired: bool,
    pub battery_percentage: Option<u8>,
}

#[derive(Clone)]
pub struct BluetoothManager {
    conn: Connection,
    adapter_path: Option<String>,
}

impl BluetoothManager {
    pub async fn new() -> zbus::Result<Self> {
        let conn = Connection::system().await?;
        let adapter_path = Self::find_adapter(&conn).await;
        Ok(Self { conn, adapter_path })
    }
    
    async fn find_adapter(conn: &Connection) -> Option<String> {
        let objects: HashMap<zbus::zvariant::OwnedObjectPath, HashMap<String, HashMap<String, zbus::zvariant::OwnedValue>>> = conn
            .call_method(
                Some("org.bluez"),
                "/",
                Some("org.freedesktop.DBus.ObjectManager"),
                "GetManagedObjects",
                &(),
            )
            .await
            .ok()?
            .body()
            .deserialize()
            .ok()
            .unwrap_or_default();
        
        for path in objects.keys() {
            if path.as_str().contains("/org/bluez/hci") && !path.as_str().contains("/dev_") {
                return Some(path.to_string());
            }
        }
        None
    }
    
    pub async fn is_available(&self) -> bool {
        self.adapter_path.is_some()
    }
    
    pub async fn is_powered(&self) -> zbus::Result<bool> {
        let adapter = self.adapter_path.as_ref()
            .ok_or_else(|| zbus::Error::Address("No Bluetooth adapter found".to_string()))?;
        
        let path: zbus::zvariant::ObjectPath = adapter.as_str().try_into()
            .map_err(|e: zbus::zvariant::Error| zbus::Error::Variant(e))?;
        let reply = self.conn
            .call_method(
                Some("org.bluez"),
                &path,
                Some("org.freedesktop.DBus.Properties"),
                "Get",
                &("org.bluez.Adapter1", "Powered"),
            )
            .await?
            .body()
            .deserialize::<zbus::zvariant::OwnedValue>()?;
        
        bool::try_from(reply).map_err(zbus::Error::from)
    }
    
    pub async fn set_powered(&self, powered: bool) -> zbus::Result<()> {
        let adapter = self.adapter_path.as_ref()
            .ok_or_else(|| zbus::Error::Address("No Bluetooth adapter found".to_string()))?;
        
        let path: zbus::zvariant::ObjectPath = adapter.as_str().try_into()
            .map_err(|e: zbus::zvariant::Error| zbus::Error::Variant(e))?;
        let value = zbus::zvariant::Value::Bool(powered);
        self.conn
            .call_method(
                Some("org.bluez"),
                &path,
                Some("org.freedesktop.DBus.Properties"),
                "Set",
                &("org.bluez.Adapter1", "Powered", value),
            )
            .await?;
        Ok(())
    }
    
    pub async fn get_devices(&self) -> zbus::Result<Vec<BluetoothDevice>> {
        let objects: HashMap<zbus::zvariant::OwnedObjectPath, HashMap<String, HashMap<String, zbus::zvariant::OwnedValue>>> = self.conn
            .call_method(
                Some("org.bluez"),
                "/",
                Some("org.freedesktop.DBus.ObjectManager"),
                "GetManagedObjects",
                &(),
            )
            .await?
            .body()
            .deserialize()?;
        
        let mut devices = Vec::new();
        
        for (path, interfaces) in objects {
            if let Some(device_props) = interfaces.get("org.bluez.Device1") {
                let name = device_props.get("Name")
                    .and_then(|v| v.downcast_ref::<String>().ok())
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| "Unknown".to_string());
                
                let address = device_props.get("Address")
                    .and_then(|v| v.downcast_ref::<String>().ok())
                    .map(|s| s.to_string())
                    .unwrap_or_default();
                
                let is_connected = device_props.get("Connected")
                    .and_then(|v| v.downcast_ref::<bool>().ok())
                    .unwrap_or(false);
                
                let is_paired = device_props.get("Paired")
                    .and_then(|v| v.downcast_ref::<bool>().ok())
                    .unwrap_or(false);
                
                let battery = device_props.get("BatteryPercentage")
                    .and_then(|v| v.downcast_ref::<u8>().ok());
                
                let device_class = match device_props.get("Class") {
                    Some(v) => v.downcast_ref::<u32>()
                        .ok()
                        .map(DeviceType::from)
                        .unwrap_or(DeviceType::Unknown),
                    None => DeviceType::Unknown,
                };
                
                devices.push(BluetoothDevice {
                    path: path.to_string(),
                    name,
                    address,
                    device_type: device_class,
                    is_connected,
                    is_paired,
                    battery_percentage: battery,
                });
            }
        }
        
        devices.sort_by(|a, b| {
            b.is_connected.cmp(&a.is_connected)
                .then_with(|| b.is_paired.cmp(&a.is_paired))
                .then_with(|| a.name.cmp(&b.name))
        });
        
        Ok(devices)
    }
    
    pub async fn start_discovery(&self) -> zbus::Result<()> {
        let adapter = self.adapter_path.as_ref()
            .ok_or_else(|| zbus::Error::Address("No Bluetooth adapter found".to_string()))?;
        
        let path: zbus::zvariant::ObjectPath = adapter.as_str().try_into()
            .map_err(|e: zbus::zvariant::Error| zbus::Error::Variant(e))?;
        self.conn
            .call_method(
                Some("org.bluez"),
                &path,
                Some("org.bluez.Adapter1"),
                "StartDiscovery",
                &(),
            )
            .await?;
        Ok(())
    }
    
    pub async fn stop_discovery(&self) -> zbus::Result<()> {
        let adapter = self.adapter_path.as_ref()
            .ok_or_else(|| zbus::Error::Address("No Bluetooth adapter found".to_string()))?;
        
        let path: zbus::zvariant::ObjectPath = adapter.as_str().try_into()
            .map_err(|e: zbus::zvariant::Error| zbus::Error::Variant(e))?;
        self.conn
            .call_method(
                Some("org.bluez"),
                &path,
                Some("org.bluez.Adapter1"),
                "StopDiscovery",
                &(),
            )
            .await?;
        Ok(())
    }
    
    pub async fn connect_device(&self, path_str: &str) -> zbus::Result<()> {
        let path: zbus::zvariant::ObjectPath = path_str.try_into()
            .map_err(|e: zbus::zvariant::Error| zbus::Error::Variant(e))?;
        self.conn
            .call_method(
                Some("org.bluez"),
                &path,
                Some("org.bluez.Device1"),
                "Connect",
                &(),
            )
            .await?;
        Ok(())
    }
    
    pub async fn disconnect_device(&self, path_str: &str) -> zbus::Result<()> {
        let path: zbus::zvariant::ObjectPath = path_str.try_into()
            .map_err(|e: zbus::zvariant::Error| zbus::Error::Variant(e))?;
        self.conn
            .call_method(
                Some("org.bluez"),
                &path,
                Some("org.bluez.Device1"),
                "Disconnect",
                &(),
            )
            .await?;
        Ok(())
    }
    
    pub async fn pair_device(&self, path_str: &str) -> zbus::Result<()> {
        let path: zbus::zvariant::ObjectPath = path_str.try_into()
            .map_err(|e: zbus::zvariant::Error| zbus::Error::Variant(e))?;
        self.conn
            .call_method(
                Some("org.bluez"),
                &path,
                Some("org.bluez.Device1"),
                "Pair",
                &(),
            )
            .await?;
        Ok(())
    }
    
    pub async fn forget_device(&self, device_path_str: &str) -> zbus::Result<()> {
        let adapter = self.adapter_path.as_ref()
            .ok_or_else(|| zbus::Error::Address("No Bluetooth adapter found".to_string()))?;
        
        let adapter_path: zbus::zvariant::ObjectPath = adapter.as_str().try_into()
            .map_err(|e: zbus::zvariant::Error| zbus::Error::Variant(e))?;
        
        let device_path: zbus::zvariant::ObjectPath = device_path_str.try_into()
            .map_err(|e: zbus::zvariant::Error| zbus::Error::Variant(e))?;
        
        self.conn
            .call_method(
                Some("org.bluez"),
                &adapter_path,
                Some("org.bluez.Adapter1"),
                "RemoveDevice",
                &device_path,
            )
            .await?;
        Ok(())
    }
}
