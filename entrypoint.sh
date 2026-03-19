#!/bin/bash
set -e

PROPS="/etc/lemoncuc/properties.json"

# ── Read display config from properties.json ─────────────────────────────────
if [ -f "$PROPS" ]; then
    SCREEN_WIDTH="$(jq -r '.display.width // 1280' "$PROPS")"
    SCREEN_HEIGHT="$(jq -r '.display.height // 800' "$PROPS")"
    SCREEN_DEPTH="$(jq -r '.display.depth // 24' "$PROPS")"
    CURSOR_THEME="$(jq -r '.cursor.theme // "Adwaita"' "$PROPS")"
    CURSOR_SIZE="$(jq -r '.cursor.size // 48' "$PROPS")"
else
    SCREEN_WIDTH="${SCREEN_WIDTH:-1280}"
    SCREEN_HEIGHT="${SCREEN_HEIGHT:-800}"
    SCREEN_DEPTH="${SCREEN_DEPTH:-24}"
    CURSOR_THEME="Adwaita"
    CURSOR_SIZE=48
fi

VNC_PORT="${VNC_PORT:-5900}"
VNC_PASSWORD="${VNC_PASSWORD:-}"

# ── Clean up child processes on exit ─────────────────────────────────────────
cleanup() {
    echo "entrypoint: shutting down..."
    kill 0 2>/dev/null || true
    wait
}
trap cleanup EXIT SIGTERM SIGINT

# ── Virtual display ───────────────────────────────────────────────────────────
Xvfb :0 -screen 0 "${SCREEN_WIDTH}x${SCREEN_HEIGHT}x${SCREEN_DEPTH}" \
    -ac +extension GLX +render -noreset &
sleep 1

# ── Disable screensaver / DPMS / blanking ─────────────────────────────────────
xset s off -dpms s noblank 2>/dev/null || true

# ── Cursor theme (big cursor) ─────────────────────────────────────────────────
export XCURSOR_THEME="$CURSOR_THEME"
export XCURSOR_SIZE="$CURSOR_SIZE"
xsetroot -cursor_name left_ptr 2>/dev/null || true

# ── D-Bus system + session bus (no systemd) ──────────────────────────────────
mkdir -p /run/dbus
dbus-daemon --system --fork 2>/dev/null || true
eval "$(dbus-launch --sh-syntax)"
export DBUS_SESSION_BUS_ADDRESS

# ── PulseAudio (virtual sound server) ─────────────────────────────────────────
pulseaudio --start --exit-idle-time=-1
pactl load-module module-null-sink sink_name=virtual_speaker \
    sink_properties=device.description="VirtualSpeaker" 2>/dev/null || true
pactl set-default-sink virtual_speaker 2>/dev/null || true

# ── sshd ──────────────────────────────────────────────────────────────────────
mkdir -p /run/sshd
/usr/sbin/sshd

# ── dunst (notification daemon) ───────────────────────────────────────────────
dunst &

# ── i3 window manager ────────────────────────────────────────────────────────
i3 &
sleep 1

# ── VNC server ────────────────────────────────────────────────────────────────
if [ -n "${VNC_PASSWORD}" ]; then
    mkdir -p ~/.vnc
    x11vnc -storepasswd "${VNC_PASSWORD}" ~/.vnc/passwd
    x11vnc -display :0 -forever -shared -rfbauth ~/.vnc/passwd -rfbport "${VNC_PORT}" &
else
    x11vnc -display :0 -forever -shared -nopw -rfbport "${VNC_PORT}" &
fi
sleep 1

# ── LemonCUC backend (axum server on :6080) ──────────────────────────────────
echo "entrypoint: starting lemon-cuc-backend"
exec lemon-cuc-backend