use zbus::Connection;
use zbus::zvariant::ObjectPath;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BluetoothDevice {
    pub path: String,
    pub name: String,
    pub device_type: Option<DeviceType>,
    pub is_connected: bool,
    pub is_paired: bool,
    pub battery_percentage: Option<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum DeviceType {
    Audio,
    Keyboard,
    Mouse,
    Phone,
}

pub struct BluetoothManager {
    conn: Connection,
    adapter_path: Option<String>,
}

impl BluetoothManager {
    pub async fn new() -> zbus::Result<Self> {
        let conn = Connection::system().await?;
        let adapter_path = Self::find_adapter(&conn).await?;
        Ok(Self { conn, adapter_path })
    }

    async fn find_adapter(conn: &Connection) -> zbus::Result<Option<String>> {
        let reply: std::collections::HashMap<zbus::zvariant::OwnedObjectPath, std::collections::HashMap<String, std::collections::HashMap<String, zbus::zvariant::OwnedValue>>> = conn
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

        for (path, interfaces) in reply {
            if interfaces.contains_key("org.bluez.Adapter1") {
                return Ok(Some(path.to_string()));
            }
        }
        Ok(None)
    }

    pub async fn is_powered(&self) -> zbus::Result<bool> {
        let adapter_str = self.adapter_path.as_ref()
            .ok_or_else(|| zbus::Error::Address("No Bluetooth adapter found".to_string()))?;
        let adapter = ObjectPath::try_from(adapter_str.as_str()).map_err(|e| zbus::Error::Variant(e))?;
        
        let reply = self.conn
            .call_method(
                Some("org.bluez"),
                &adapter,
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
        let adapter_str = self.adapter_path.as_ref()
            .ok_or_else(|| zbus::Error::Address("No Bluetooth adapter found".to_string()))?;
        let adapter = ObjectPath::try_from(adapter_str.as_str()).map_err(|e| zbus::Error::Variant(e))?;
        
        let value = zbus::zvariant::Value::Bool(powered);
        self.conn
            .call_method(
                Some("org.bluez"),
                &adapter,
                Some("org.freedesktop.DBus.Properties"),
                "Set",
                &("org.bluez.Adapter1", "Powered", value),
            )
            .await?;
        Ok(())
    }

    pub async fn start_discovery(&self) -> zbus::Result<()> {
        let adapter_str = self.adapter_path.as_ref()
            .ok_or_else(|| zbus::Error::Address("No Bluetooth adapter found".to_string()))?;
        let adapter = ObjectPath::try_from(adapter_str.as_str()).map_err(|e| zbus::Error::Variant(e))?;
        
        self.conn
            .call_method(
                Some("org.bluez"),
                &adapter,
                Some("org.bluez.Adapter1"),
                "StartDiscovery",
                &(),
            )
            .await?;
        Ok(())
    }

    pub async fn stop_discovery(&self) -> zbus::Result<()> {
        let adapter_str = self.adapter_path.as_ref()
            .ok_or_else(|| zbus::Error::Address("No Bluetooth adapter found".to_string()))?;
        let adapter = ObjectPath::try_from(adapter_str.as_str()).map_err(|e| zbus::Error::Variant(e))?;
        
        self.conn
            .call_method(
                Some("org.bluez"),
                &adapter,
                Some("org.bluez.Adapter1"),
                "StopDiscovery",
                &(),
            )
            .await?;
        Ok(())
    }

    pub async fn get_devices(&self) -> zbus::Result<Vec<BluetoothDevice>> {
        let reply: std::collections::HashMap<zbus::zvariant::OwnedObjectPath, std::collections::HashMap<String, std::collections::HashMap<String, zbus::zvariant::OwnedValue>>> = self.conn
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
        for (path, interfaces) in reply {
            if let Some(props) = interfaces.get("org.bluez.Device1") {
                let name = props.get("Name")
                    .or_else(|| props.get("Alias"))
                    .and_then(|v| <&str>::try_from(v).ok())
                    .unwrap_or("Unknown Device")
                    .to_string();

                let is_connected = props.get("Connected")
                    .and_then(|v| bool::try_from(v).ok())
                    .unwrap_or(false);

                let is_paired = props.get("Paired")
                    .and_then(|v| bool::try_from(v).ok())
                    .unwrap_or(false);

                let battery_percentage = props.get("BatteryPercentage")
                    .and_then(|v| u8::try_from(v).ok());

                let icon = props.get("Icon")
                    .and_then(|v| <&str>::try_from(v).ok());

                let device_type = match icon {
                    Some("audio-card") | Some("audio-speakers") | Some("audio-headset") | Some("audio-headphones") => Some(DeviceType::Audio),
                    Some("input-keyboard") => Some(DeviceType::Keyboard),
                    Some("input-mouse") | Some("input-tablet") => Some(DeviceType::Mouse),
                    Some("phone") => Some(DeviceType::Phone),
                    _ => None,
                };

                devices.push(BluetoothDevice {
                    path: path.to_string(),
                    name,
                    device_type,
                    is_connected,
                    is_paired,
                    battery_percentage,
                });
            }
        }

        devices.sort_by(|a, b| b.is_connected.cmp(&a.is_connected).then_with(|| b.is_paired.cmp(&a.is_paired)).then_with(|| a.name.cmp(&b.name)));
        Ok(devices)
    }

    pub async fn connect_device(&self, path: &str) -> zbus::Result<()> {
        let p = ObjectPath::try_from(path).map_err(|e| zbus::Error::Variant(e))?;
        self.conn
            .call_method(
                Some("org.bluez"),
                &p,
                Some("org.bluez.Device1"),
                "Connect",
                &(),
            )
            .await?;
        Ok(())
    }

    pub async fn disconnect_device(&self, path: &str) -> zbus::Result<()> {
        let p = ObjectPath::try_from(path).map_err(|e| zbus::Error::Variant(e))?;
        self.conn
            .call_method(
                Some("org.bluez"),
                &p,
                Some("org.bluez.Device1"),
                "Disconnect",
                &(),
            )
            .await?;
        Ok(())
    }

    pub async fn pair_device(&self, path: &str) -> zbus::Result<()> {
        let p = ObjectPath::try_from(path).map_err(|e| zbus::Error::Variant(e))?;
        self.conn
            .call_method(
                Some("org.bluez"),
                &p,
                Some("org.bluez.Device1"),
                "Pair",
                &(),
            )
            .await?;
        Ok(())
    }

    pub async fn forget_device(&self, path: &str) -> zbus::Result<()> {
        let adapter_str = self.adapter_path.as_ref()
            .ok_or_else(|| zbus::Error::Address("No Bluetooth adapter found".to_string()))?;
        let adapter = ObjectPath::try_from(adapter_str.as_str()).map_err(|e| zbus::Error::Variant(e))?;
        
        let path_obj = ObjectPath::try_from(path)
            .map_err(|e| zbus::Error::Variant(e))?;

        self.conn
            .call_method(
                Some("org.bluez"),
                &adapter,
                Some("org.bluez.Adapter1"),
                "RemoveDevice",
                &(path_obj),
            )
            .await?;
        Ok(())
    }
}
