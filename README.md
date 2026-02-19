# Orbit - WiFi/Bluetooth Manager for Wayland

A native WiFi/Bluetooth manager for Wayland using Rust, GTK4, and layer-shell with a modern glassmorphism UI.

![Orbit Screenshot](docs/screenshot.png)

## Features

- **WiFi Management**
  - Scan and list available networks
  - Connect to open and secured networks
  - Disconnect from networks
  - Saved networks with auto-connect toggle
  - Network details (IP, gateway, DNS, MAC, speed)

- **Bluetooth Management**
  - Scan for devices
  - Pair, connect, disconnect devices
  - Forget paired devices

- **Modern UI**
  - Glassmorphism design with transparency
  - Slide-up overlay dialogs
  - Pill-style tabs
  - Theme customization

- **Daemon Mode**
  - Background daemon for quick access
  - Toggle visibility with `orbit toggle`

## Requirements

- Wayland compositor with layer-shell support (sway, hyprland, etc.)
- NetworkManager
- BlueZ
- GTK4
- gtk4-layer-shell

## Installation

### Arch Linux (AUR)

```bash
paru -S orbit-wifi
# or
yay -S orbit-wifi
```

### From Source

```bash
git clone https://github.com/yourusername/orbit.git
cd orbit
cargo build --release
sudo install -Dm755 target/release/orbit /usr/bin/orbit
```

### Dependencies

```bash
# Arch Linux
sudo pacman -S gtk4 gtk4-layer-shell networkmanager bluez

# Fedora
sudo dnf install gtk4 gtk4-layer-shell NetworkManager bluez
```

## Usage

```bash
# Launch GUI
orbit

# Run as background daemon
orbit daemon &

# Toggle daemon visibility
orbit toggle

# List WiFi networks (CLI)
orbit list
```

## Configuration

### Config File (`~/.config/orbit/config.toml`)

```toml
position = "top-right"  # top-left, top, top-right, left, center, right, bottom-left, bottom, bottom-right
margin_top = 10
margin_bottom = 10
margin_left = 10
margin_right = 10
```

### Theme File (`~/.config/orbit/theme.toml`)

```toml
accent_primary = "#8B5CF6"    # Violet
accent_secondary = "#D97706"  # Gold
```

Orbit uses a built-in glassmorphism theme with violet and gold accents by default. Override the colors by creating `~/.config/orbit/theme.toml`.

## Waybar Integration

Add to your `~/.config/waybar/config`:

```json
"custom/orbit": {
    "exec": "orbit toggle",
    "on-click": "orbit toggle",
    "tooltip": false
}
```

## License

MIT License - see [LICENSE](LICENSE) for details.

## Credits

UI designs created with [Stitch](https://stitch.design).
