#!/bin/bash

# sync-orbit-theme.sh - Bridge between system theme switcher and Orbit

ORBIT_THEME_DIR="$HOME/.config/orbit"
ORBIT_THEME_FILE="$ORBIT_THEME_DIR/theme.toml"

# Function to trigger reload
trigger_reload() {
    if command -v orbit &> /dev/null; then
        orbit reload-theme 2>/dev/null
    elif [ -f "$HOME/Documents/projects/orbit/target/release/orbit" ]; then
        "$HOME/Documents/projects/orbit/target/release/orbit" reload-theme 2>/dev/null
    fi
}

# Handle Default/Reset
if [ "$1" == "default" ] || [ "$1" == "reset" ]; then
    rm -f "$ORBIT_THEME_FILE"
    trigger_reload
    exit 0
fi

# Handle Normal Sync
if [ "$#" -lt 4 ]; then
    echo "Usage: $0 <accent_primary> <accent_secondary> <background> <foreground>"
    echo "       $0 default"
    exit 1
fi

ACCENT_PRIMARY="$1"
ACCENT_SECONDARY="$2"
BACKGROUND="$3"
FOREGROUND="$4"

# Ensure directory exists
mkdir -p "$ORBIT_THEME_DIR"

# Write theme.toml
cat > "$ORBIT_THEME_FILE" <<EOF
accent_primary = "${ACCENT_PRIMARY}"
accent_secondary = "${ACCENT_SECONDARY}"
background = "${BACKGROUND}"
foreground = "${FOREGROUND}"
EOF

# Trigger hot-reload
trigger_reload
