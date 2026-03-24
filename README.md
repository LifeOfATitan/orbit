# Orbit - WiFi/Bluetooth/VPN Manager for Wayland

A native network manager for Wayland using Rust, GTK4, and layer-shell with a high-contrast glassmorphism UI.

## Interface Preview

<p align="center">
  <img src="screenshots/orbit_demo.gif" width="800" alt="Orbit Main Interface">
</p>

## Features

- **WiFi Management**
  - **Smart Search**: Real-time filtering that prioritizes start-of-word matches.
  - Scan and list available networks with GTK signal strength icons.
  - Connect to open and secured networks (WPA2/WPA3 support).
  - Disconnect from active networks.
  - **Integrated Saved Networks**: Manage autoconnect and forget profiles via a sleek footer overlay.
  - Detailed network information (IPv4/IPv6, Gateway, DNS, MAC, Speed).
- **Bluetooth Management**
  - **Device Details**: Show MAC address, trust status, and signal strength (RSSI).
  - **Battery & Charging**: Real-time battery levels with low-battery alerts and charging indicators.
  - **Pairing Agent**: Built-in support for PIN/Passkey entry and numeric confirmation.
  - Scan, pair, connect, disconnect, and remove/forget devices.
- **VPN & Privacy Dashboard**
  - **Privacy Dashboard**: Real-time Public IP and ISP detection.
  - **DNS Identification**: Automatically identifies DNS providers (Local Router, Cloudflare, Google, etc.).
  - **Unified VPN Control**: Manage NetworkManager profiles and external apps (Riseup, Tailscale, Mullvad).
- **Modern UI/UX**
  - **High-Contrast Glassmorphism**: High-quality translucent panels with customizable opacity.
  - **Smooth Transitions**: Animated slide-up overlays for passwords, details, and errors.
  - **Dynamic Positioning**: Anchor to any corner or center edge via CLI.
  - **Keyboard Friendly**: `Escape` key support to close overlays or hide the window.
- **Theme Synchronization**
  - **Hot-Reloading**: Change colors in real-time without restarting the application.
  - Automatically syncs with system background, foreground, and accent colors.

## Requirements

- Wayland compositor with layer-shell support (Hyprland, Sway, etc.)
- NetworkManager
- BlueZ
- GTK4 & gtk4-layer-shell

## Installation

### Arch Linux (AUR)

```bash
# Using paru
paru -S orbit-wifi

# Using yay
yay -S orbit-wifi
```

After installation, enable the background daemon:
```bash
systemctl --user enable --now orbit
```

### From Source

```bash
git clone https://github.com/LifeOfATitan/orbit.git
cd orbit
cargo build --release
sudo install -Dm755 target/release/orbit /usr/bin/orbit
# Install systemd service
mkdir -p ~/.config/systemd/user/
cp orbit.service ~/.config/systemd/user/
systemctl --user daemon-reload
systemctl --user enable --now orbit
```

## Usage

```bash
# Toggle visibility
orbit toggle [position]

# Open specific tab directly
orbit toggle --tab [wifi|bluetooth|vpn]

# Show the window
orbit show

# Hide the window
orbit hide

# Output status in JSON for Waybar
orbit waybar-status

# Manually trigger a theme reload
orbit reload-theme

# Reload config without restarting
orbit reload-config

# Run as daemon (handled automatically by systemd)
orbit daemon

# List WiFi networks in terminal
orbit list
```

## Waybar Integration

Orbit is designed to look native in your bar. Add the following module to your Waybar `config.jsonc`:

```jsonc
"custom/orbit": {
    "exec": "orbit waybar-status",
    "return-type": "json",
    "interval": 10,
    "on-click": "orbit toggle top-right",
    "format": "󱗿"
}
```

## Configuration

### Config File (`~/.config/orbit/config.toml`)

```toml
position = "top-right"
margin_top = 10
margin_bottom = 10
margin_left = 10
margin_right = 10
```

### Theme File (`~/.config/orbit/theme.toml`)

```toml
accent_primary = "#8b5cf6"
accent_secondary = "#06b6d4"
background = "#1e1e2e"
foreground = "#d4d4d8"
```

## License

MIT License - see [LICENSE](LICENSE) for details.

## Credits

Developed by [LifeOfATitan](https://github.com/LifeOfATitan).
