# Orbit - WiFi/Bluetooth Manager for Wayland

A native WiFi/Bluetooth manager for Wayland using Rust, GTK4, and layer-shell.

## Project Structure

```
~/Documents/projects/orbit/
├── Cargo.toml                 # Dependencies
├── AGENTS.md                  # This file
├── README.md                  # User documentation
├── LICENSE                    # MIT license
├── src/
│   ├── main.rs               # CLI entry point (commands: list, daemon, toggle, or no args for GUI)
│   ├── lib.rs                # Module exports
│   ├── config.rs             # TOML config loader (position, margins)
│   ├── theme.rs              # TOML theme loader with built-in default glassmorphism
│   ├── dbus/
│   │   ├── mod.rs            # Re-exports
│   │   ├── network_manager.rs # NetworkManager D-Bus client (WiFi + saved networks)
│   │   └── bluez.rs          # BlueZ D-Bus client (Bluetooth)
│   ├── ui/
│   │   ├── mod.rs            # Re-exports
│   │   ├── window.rs         # Layer-shell window with Overlay for slide-up dialogs
│   │   ├── header.rs         # Pill-style tabs + power toggle
│   │   ├── network_list.rs   # WiFi list with sections (Active/Available), glass cards
│   │   ├── device_list.rs    # Bluetooth list with sections (Connected/Paired/Available)
│   │   ├── saved_networks_list.rs # Saved WiFi networks with autoconnect toggles
│   │   └── password_dialog.rs # Unused (inline dialog in window.rs)
│   └── app/
│       ├── mod.rs            # Application setup, event handling, threading
│       └── daemon.rs         # Unix socket IPC for daemon mode
└── aur/                      # AUR package files
    ├── PKGBUILD              # Arch package build script
    ├── .SRCINFO              # AUR metadata
    └── orbit.install         # Post-install instructions
```

## Build & Run

```bash
# Build
cargo build --release

# Run GUI directly
./target/release/orbit

# Run as daemon (background, toggle visibility)
./target/release/orbit daemon &
./target/release/orbit toggle  # Show/hide window

# List WiFi networks (CLI)
./target/release/orbit list
```

## Dependencies

- gtk4 = "0.9"
- gtk4-layer-shell = "0.4"
- zbus = { version = "4.4", features = ["tokio"] }
- tokio = { version = "1", features = ["full"] }
- async-channel = "2"
- serde, toml, clap, uuid

## Architecture

### Threading Model

The app uses a hybrid threading model:
1. GTK4 runs on the main thread with glib event loop
2. D-Bus calls run in separate threads with their own Tokio runtime
3. Communication between threads uses `async_channel` and `glib::spawn_future_local`

```
Main Thread (GTK/glib)          Worker Threads (Tokio)
     |                                    |
     | glib::spawn_future_local()         |
     | <-- async_channel::Receiver        |
     |                                    | std::thread::spawn()
     |                                    | --> rt.block_on(async {...})
     | async_channel::Sender.send()  <----|
```

### AppEvent Types

```rust
enum AppEvent {
    WifiScanResult(Vec<AccessPoint>),
    SavedNetworksResult(Vec<SavedNetwork>),
    NetworkDetailsResult(NetworkDetails),
    BtScanResult(Vec<BluetoothDevice>),
    WifiPowerState(bool),
    BtPowerState(bool),
    Error(String),
    DaemonCommand(DaemonCommand),
    RefreshRequest,
}
```

### D-Bus Integration

**NetworkManager:**
- Service: `org.freedesktop.NetworkManager`
- Key methods: `GetDevices`, `GetAllAccessPoints`, `RequestScan`, `AddAndActivateConnection`, `DeactivateConnection`
- Properties: `WirelessEnabled`, `ActiveConnections`
- Settings: `ListConnections`, `GetSettings`, `Update`, `Delete`

**BlueZ:**
- Service: `org.bluez`
- Key methods: `StartDiscovery`, `StopDiscovery`, `Connect`, `Disconnect`, `Pair`, `RemoveDevice`
- Uses `ObjectManager.GetManagedObjects` to enumerate devices

### zbus v4 Patterns

```rust
// Direct deserialization for arrays
let devices: Vec<OwnedObjectPath> = conn
    .call_method(...)
    .await?
    .body()
    .deserialize()?;

// ObjectPath from string
let path: ObjectPath = str.try_into()
    .map_err(|e: zvariant::Error| zbus::Error::Variant(e))?;

// Property Get returns OwnedValue
let reply: OwnedValue = conn.call_method(...).await?.body().deserialize()?;
let value: bool = bool::try_from(reply).map_err(zbus::Error::from)?;
```

## Theme Integration

Orbit uses a glassmorphism design with a violet and gold color scheme:

| Token | Value | Usage |
|-------|-------|-------|
| Panel background | `rgba(15, 15, 20, 0.75)` | Main window |
| Card background | `rgba(255, 255, 255, 0.08)` | Network/device rows |
| Card hover background | `rgba(255, 255, 255, 0.05)` | Hover state |
| Card hover border | `rgba(217, 119, 6, 0.35)` | Gold border glow |
| Primary accent | `#8B5CF6` (violet) | Buttons, icons |
| Secondary accent | `#D97706` (gold) | Section headers, toggles, highlights |
| Connected gradient | `rgba(139,92,246,0.15) → rgba(217,119,6,0.1)` | Active items |
| Connected hover border | `rgba(139, 92, 246, 0.4)` | Violet border on hover |
| Overlay background | `rgba(10, 10, 15, 0.98)` | Dialog overlays |
| Power toggle ON | `rgba(217, 119, 6, 0.9)` | Gold when enabled |
| Power toggle OFF | `rgba(100, 100, 100, 0.5)` | Gray when disabled |
| Error | `#EF4444` | Error dialogs/badges |

Theme file at `~/.config/orbit/theme.toml`:

```toml
accent_primary = "#8B5CF6"    # Violet
accent_secondary = "#D97706"  # Gold
```

If no theme file exists, Orbit uses the built-in default glassmorphism theme.

### Theme Loading

```rust
// In theme.rs
pub fn load() -> Self {
    let theme_path = Self::theme_path();
    
    if theme_path.exists() {
        // Load from ~/.config/orbit/theme.toml
        match toml::from_str::<ThemeFile>(&content) {
            Ok(theme_file) => { /* use custom colors */ }
            Err(_) => { /* fallback to default */ }
        }
    }
    
    Self::default() // Built-in glassmorphism default
}
```

## Configuration

Config file at `~/.config/orbit/config.toml`:

```toml
position = "top-right"  # top-left, top, top-right, left, center, right, bottom-left, bottom, bottom-right
margin_top = 10
margin_bottom = 10
margin_left = 10
margin_right = 10
```

## Completed Features

**WiFi:**
- ✅ Scan and list available networks with GTK signal strength icons
- ✅ Connect to open and secured networks (overlay password dialog)
- ✅ Disconnect from networks
- ✅ Saved networks tab with forget functionality
- ✅ Auto-connect toggle per saved network
- ✅ Network details overlay with icons (IP, gateway, DNS, MAC, speed)
- ✅ Security lock icon for encrypted networks
- ✅ Section headers: "ACTIVE CONNECTION", "AVAILABLE NETWORKS"

**Bluetooth:**
- ✅ Scan and list devices with GTK device type icons
- ✅ Pair, connect, disconnect devices
- ✅ Forget/remove paired devices
- ✅ Section headers: "CONNECTED", "PAIRED", "AVAILABLE"

**UI/UX:**
- ✅ Glassmorphism design with semi-transparent backgrounds
- ✅ Pill-style tabs with smooth transitions (only active tab has rounded corners)
- ✅ Power toggle for WiFi and Bluetooth (gold when ON, hidden on Saved tab)
- ✅ Consistent window size - overlays don't resize window
- ✅ Slide-up overlay dialogs with Revealer animations:
  - Password dialog for WiFi connection
  - Error dialog for failures
  - Details panel for network info with icons
- ✅ Theme customization via TOML
- ✅ GTK symbolic icons throughout
- ✅ Icon containers for connected items (circular violet background)
- ✅ Card hover with border glow and shadow effect
- ✅ Details panel with icons for each row

**Daemon Mode:**
- ✅ Background daemon with Unix socket IPC
- ✅ Toggle visibility via `orbit toggle`
- ✅ Socket at `$XDG_RUNTIME_DIR/orbit.sock`

**Real-time Updates:**
- ✅ 10-second periodic refresh (only when visible)
- ✅ Refresh on show/toggle
- ✅ Power state tracked per tab

## In Progress

- Bluetooth PIN pairing dialog (requires BlueZ Agent registration)

## Remaining Features

- VPN connection support (list and connect)
- Keyboard shortcuts (Escape to close, Ctrl+R to refresh)
- D-Bus signal monitoring for instant updates (instead of polling)

## Key Patterns

### List Container (Box, not ListBox)

**IMPORTANT:** Orbit uses `gtk::Box` for list containers, NOT `gtk::ListBox`. 

GTK's `ListBox` automatically wraps each child in a `ListBoxRow` which has built-in hover styling that cannot be fully overridden. Using `Box` gives full control over styling.

```rust
// In network_list.rs, device_list.rs, saved_networks_list.rs
let list_box = gtk::Box::builder()
    .orientation(Orientation::Vertical)
    .css_classes(["orbit-list"])
    .build();
```

### Tab Styling

Tabs use the `flat` CSS class to remove GTK's default button frame, and only the active tab gets rounded corners:

```css
.orbit-tab {{
    background: transparent;
    background-image: none;
    color: rgba(255, 255, 255, 0.5);
    border-radius: 0;           /* No rounded corners for inactive */
    border: none;
    box-shadow: none;
    outline: none;
    -gtk-icon-shadow: none;
    text-shadow: none;
}}

.orbit-tab:hover {{
    background: transparent;
    background-image: none;
    color: rgba(255, 255, 255, 0.75);
    box-shadow: none;
    border: none;
}}

.orbit-tab.active {{
    background: rgba(255, 255, 255, 0.15);
    border-radius: 9999px;      /* Only active gets rounded */
    color: #ffffff;
}}
```

Tab buttons also use the `flat` CSS class:

```rust
let wifi_tab = gtk::Button::builder()
    .label("WiFi")
    .css_classes(["orbit-tab", "flat", "active"])
    .build();
```

### Button Styling (Override GTK Defaults)

GTK buttons have default styling (shadows, background images) that must be explicitly overridden:

```css
.orbit-button {{
    background: rgba(255, 255, 255, 0.05);
    background-image: none;     /* Override GTK gradient */
    border: 1px solid rgba(255, 255, 255, 0.05);
    border-radius: 9999px;
    box-shadow: none;           /* Override GTK shadow */
    /* ... */
}}

.orbit-button:hover {{
    background: rgba(217, 119, 6, 0.15);
    background-image: none;
    box-shadow: none;
}}

.orbit-button.primary {{
    background: #8B5CF6;
    background-image: none;
    box-shadow: 0 4px 12px rgba(139, 92, 246, 0.4);  /* Violet glow */
}}

.orbit-button.primary:hover {{
    background: color-mix(in srgb, #8B5CF6 85%, white);
    background-image: none;
    box-shadow: 0 4px 16px rgba(139, 92, 246, 0.5);
}}
```

### Glass Cards

Network and device rows use glassmorphism styling with border glow on hover:

```css
.orbit-network-row {{
    background: rgba(255, 255, 255, 0.08);
    border: 1px solid rgba(255, 255, 255, 0.05);
    border-radius: 12px;
    transition: all 0.2s ease;
}}

.orbit-network-row:hover {{
    background: rgba(255, 255, 255, 0.05);
    border-color: rgba(217, 119, 6, 0.35);  /* Gold border glow */
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.2), 0 0 0 1px rgba(217, 119, 6, 0.1);
}}

.orbit-network-row.connected {{
    background: linear-gradient(135deg, rgba(139, 92, 246, 0.15), rgba(217, 119, 6, 0.1));
    border: 1px solid rgba(139, 92, 246, 0.3);
}}

.orbit-network-row.connected:hover {{
    border-color: rgba(139, 92, 246, 0.4);  /* Violet border glow */
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.2), 0 0 0 1px rgba(139, 92, 246, 0.15);
}}
```

### Overlay Dialogs (Slide-up)

All dialogs use `gtk::Overlay` with `gtk::Revealer` for slide-up animations:

```rust
// In window.rs
let overlay = Overlay::new();
overlay.set_child(Some(&main_box));

let details_revealer = gtk::Revealer::builder()
    .child(&details_box)
    .reveal_child(false)
    .transition_type(gtk::RevealerTransitionType::SlideUp)
    .transition_duration(250)
    .valign(gtk::Align::End)
    .build();

overlay.add_overlay(&details_revealer);
```

Key benefits:
- No window resize or position shift
- Smooth slide-up animation
- Overlays appear on top of content

### Details Panel Icons

Each row in the network details panel has an icon:

```rust
let rows: [(&str, &str, &str); 6] = [
    ("SSID", details.ssid.as_str(), "network-wireless-symbolic"),
    ("IP Address", ip_text, "network-server-symbolic"),
    ("Gateway", gateway_text, "network-server-symbolic"),
    ("DNS", dns_text.as_str(), "web-browser-symbolic"),
    ("MAC Address", mac_text, "dialog-password-symbolic"),
    ("Speed", speed_text, "network-transmit-receive-symbolic"),
];
```

### Section Headers

Lists are organized into sections with gold uppercase headers:

```rust
let section_header = gtk::Label::builder()
    .label("ACTIVE CONNECTION")
    .css_classes(["orbit-section-header"])
    .halign(gtk::Align::Start)
    .build();
```

```css
.orbit-section-header {{
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 0.1em;
    color: #D97706;  /* Gold */
    font-weight: 700;
}}
```

### Power State Management

The power switch is shared between WiFi and Bluetooth tabs. Key points:
- `set_power_state()` sets `is_programmatic_update` flag to prevent callback firing
- Power events only update the switch when on the corresponding tab
- Saved tab hides the power switch entirely

```rust
// In header.rs
pub fn set_power_state(&self, enabled: bool) {
    *self.is_programmatic_update.borrow_mut() = true;
    self.power_switch.set_active(enabled);
    *self.is_programmatic_update.borrow_mut() = false;
}
```

### GTK Icon Names

```rust
// WiFi signal strength
fn get_signal_icon_name(strength: u8) -> &'static str {
    match strength {
        0..=24 => "network-wireless-signal-weak-symbolic",
        25..=49 => "network-wireless-signal-ok-symbolic",
        50..=74 => "network-wireless-signal-good-symbolic",
        _ => "network-wireless-signal-excellent-symbolic",
    }
}

// Security
"system-lock-screen-symbolic"

// Bluetooth device types
"smartphone-symbolic", "computer-symbolic", "audio-headphones-symbolic",
"input-keyboard-symbolic", "input-mouse-symbolic", "bluetooth-symbolic"

// Details panel
"network-wireless-symbolic", "network-server-symbolic", "web-browser-symbolic",
"dialog-password-symbolic", "network-transmit-receive-symbolic"
```

### NetworkManager Connection Update

When updating a connection setting (like autoconnect), you must provide ALL settings, not just the changed field:

```rust
// 1. Get current settings with GetSettings
// 2. Convert OwnedValue to Value
// 3. Modify the specific field
// 4. Update with complete settings dict
```

### Network List Deduplication

NetworkManager may return duplicate SSIDs. The deduplication preserves `is_connected` flag:

```rust
let mut seen_ssids: HashSet<String> = HashSet::new();
let mut unique_aps: Vec<AccessPoint> = Vec::new();

for ap in access_points {
    if !seen_ssids.contains(&ap.ssid) {
        seen_ssids.insert(ap.ssid.clone());
        unique_aps.push(ap);
    } else if ap.is_connected {
        // Preserve connected flag when merging duplicates
        if let Some(existing) = unique_aps.iter_mut().find(|x| x.ssid == ap.ssid) {
            existing.is_connected = true;
        }
    }
}
```

### Consistent Window Size

To prevent window resizing:
- Set `min_content_height` on ScrolledWindows
- Use `vexpand` and `hexpand` on containers
- Set `size_request` on the Stack
- Use Overlay for dialogs (no layout reflow)

## AUR Publishing

### Package Structure

```
aur/
├── PKGBUILD       # Arch package build script
├── .SRCINFO       # AUR metadata (generated from PKGBUILD)
└── orbit.install  # Post-install instructions
```

### Publishing Steps

1. **Create GitHub repository:**
   ```bash
   git init
   git add .
   git commit -m "Initial release v0.1.0"
   git remote add origin https://github.com/YOURUSERNAME/orbit.git
   git push -u origin main
   git tag v0.1.0
   git push --tags
   ```

2. **Update AUR files** with your GitHub URL:
   - Edit `aur/PKGBUILD` - replace `yourusername` with your GitHub username
   - Update maintainer email

3. **Generate .SRCINFO:**
   ```bash
   cd aur
   makepkg --printsrcinfo > .SRCINFO
   ```

4. **Test build locally:**
   ```bash
   cd aur
   makepkg -si
   ```

5. **Publish to AUR:**
   ```bash
   aurpublish orbit-wifi
   ```

### PKGBUILD Template

```bash
# Maintainer: Your Name <your-email@example.com>
pkgname=orbit-wifi
pkgver=0.1.0
pkgrel=1
pkgdesc="A WiFi/Bluetooth manager for Wayland with glassmorphism UI"
arch=('x86_64')
url="https://github.com/YOURUSERNAME/orbit"
license=('MIT')
install=orbit.install
depends=(
    'gtk4'
    'gtk4-layer-shell'
    'networkmanager'
    'bluez'
)
makedepends=(
    'cargo'
    'rust'
)
source=("$pkgname-$pkgver.tar.gz::https://github.com/YOURUSERNAME/orbit/archive/v$pkgver.tar.gz")
sha256sums=('SKIP')
```

## Development Notes

- GTK widgets are NOT Send/Sync - must use channels for cross-thread communication
- zbus requires a Tokio runtime context for async operations
- layer-shell only works on Wayland compositors that support it (sway, hyprland, etc.)
- Never pass GTK widgets into `std::thread::spawn` - only pass Arc<Mutex<T>> for D-Bus clients
- Overlays use `can_target(true)` to receive click events
- Theme uses built-in default if `~/.config/orbit/theme.toml` doesn't exist
- Use `gtk::Box` instead of `gtk::ListBox` for list containers to avoid GTK's built-in hover styling
- Always add `flat` CSS class to custom-styled buttons to remove GTK's default frame
- Override GTK button defaults with `background-image: none; box-shadow: none; outline: none;`

## Stitch Design Reference

UI designs were created in Stitch project `8896581497388303818`:
- WiFi Tab
- Saved Networks Tab  
- Bluetooth Tab
- Password Dialog
- Network Details Overlay
- Error Dialog
